use base64::{Engine, prelude::BASE64_STANDARD};
use flume::{Receiver, RecvError, SendError, Sender, bounded};
use inquire::{Confirm, Select};
use moka::{
    PredicateError,
    sync::{Cache, PredicateId},
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::{HashMap, VecDeque},
    ffi::OsStr,
    fs::OpenOptions,
    iter::once_with,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, SystemTime},
};
use tokio::runtime::Runtime;

use crate::{
    apis::{
        api::WebApi,
        dtos::{NodeDto, PostForFileApi, PutForFileApi},
    },
    from_dto_time, to_dto_time,
    ui::file_size::FileSize,
};

pub struct NeptisFS {
    api: Arc<RwLock<Option<WebApi>>>,
    rt: Arc<Runtime>,
    cache_lookup: Cache<PathBuf, Vec<FsNode>>,
    cache_dump: Cache<PathBuf, Arc<Vec<u8>>>,
}

#[derive(Clone, Debug)]
pub struct FsNode {
    pub path: PathBuf,
    pub attr: GenericFileAttr,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum GenericFileType {
    /// Named pipe (S_IFIFO)
    NamedPipe,
    /// Character device (S_IFCHR)
    CharDevice,
    /// Block device (S_IFBLK)
    BlockDevice,
    /// Directory (S_IFDIR)
    Directory,
    /// Regular file (S_IFREG)
    RegularFile,
    /// Symbolic link (S_IFLNK)
    Symlink,
    /// Unix domain socket (S_IFSOCK)
    Socket,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GenericFileAttr {
    /// Size in bytes
    pub size: u64,
    /// Size in blocks
    pub blocks: u64,
    /// Time of last access
    pub atime: SystemTime,
    /// Time of last modification
    pub mtime: SystemTime,
    /// Time of last metadata change
    pub ctime: SystemTime,
    /// Time of creation (macOS only)
    pub crtime: SystemTime,
    /// Kind of file (directory, file, pipe, etc.)
    pub kind: GenericFileType,
    /// Permissions
    pub perm: u16,
    /// Number of hard links
    pub nlink: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Device ID (if special file)
    pub rdev: u32,
    /// Flags (macOS only; see chflags(2))
    pub flags: u32,
}

const BLOCK_SIZE: u64 = 4096;
const FS_DURATION: Duration = Duration::from_secs(0);
const MAX_CACHE_SIZE: u64 = 1024 * 1024 * 1024;

impl NeptisFS {
    pub fn new(api: Arc<RwLock<Option<WebApi>>>, rt: Arc<Runtime>) -> Self {
        let cache_dump = Cache::builder()
            .support_invalidation_closures()
            .weigher(|_, value: &Arc<Vec<u8>>| -> u32 {
                value.len().try_into().unwrap_or(u32::MAX)
            })
            .max_capacity(MAX_CACHE_SIZE)
            .time_to_live(Duration::from_secs(10))
            .build();
        let cache_lookup = Cache::builder()
            .support_invalidation_closures()
            .max_capacity(MAX_CACHE_SIZE)
            .time_to_live(Duration::from_secs(10))
            .build();
        NeptisFS {
            api,
            rt,
            cache_dump,
            cache_lookup,
        }
    }

    fn delete_cache(&self, path: &Path) {
        let def = PathBuf::from("/");
        let parent = path.parent().unwrap_or(&def).to_path_buf();

        let p1 = parent.clone();

        let _ = self
            .cache_dump
            .invalidate_entries_if(move |x, _| x.starts_with(&p1));

        let p2 = parent.clone();
        let _ = self
            .cache_lookup
            .invalidate_entries_if(move |x, _| x.starts_with(&p2));
    }

    fn generic_dir_attr() -> GenericFileAttr {
        GenericFileAttr {
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: GenericFileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
        }
    }
    fn to_attr(node: &NodeDto) -> GenericFileAttr {
        GenericFileAttr {
            size: node.bytes,
            blocks: node.bytes / BLOCK_SIZE,
            atime: from_dto_time!(node.atime),
            mtime: from_dto_time!(node.mtime),
            ctime: from_dto_time!(node.ctime),
            crtime: from_dto_time!(node.ctime),
            kind: if node.is_dir {
                GenericFileType::Directory
            } else {
                GenericFileType::RegularFile
            },
            perm: 0o755,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
        }
    }

    pub fn do_find(&self, path: &Path) -> Result<FsNode, i32> {
        if path.parent().is_none() {
            return Ok(FsNode {
                path: path.to_path_buf(),
                attr: Self::generic_dir_attr(),
            }); // root returns this
        }
        let def = PathBuf::from("/");
        let parent = path.parent().unwrap_or(&def);
        let name = path.file_name().ok_or(libc::ENOENT)?;
        self.do_readdir(parent)
            .ok_or(libc::ENETUNREACH)?
            .into_iter()
            .find(|x| x.path == name)
            .ok_or(libc::ENOENT)
    }

    // WORKING 5-3-25
    pub fn do_readdir(&self, path: &Path) -> Option<Vec<FsNode>> {
        let mut output = Vec::new();

        // Always include "." and ".." entries (relative paths)
        output.push(FsNode {
            path: PathBuf::from("."), // Relative path!
            attr: Self::generic_dir_attr(),
        });
        output.push(FsNode {
            path: PathBuf::from(".."), // Relative path!
            attr: Self::generic_dir_attr(),
        });

        let p_str = path.to_str().unwrap();
        let ret = {
            if let Some(x) = self.cache_lookup.get(path) {
                Some(x)
            } else {
                let m_api = &*self.api.read().unwrap();
                if let Some(api) = m_api {
                    if let Ok(entries) = self.rt.block_on(async { api.browse_file(p_str).await }) {
                        let mut map: HashMap<PathBuf, Vec<FsNode>> = HashMap::new();
                        let mut ret = None;
                        for (k, v) in entries
                            .into_iter()
                            .filter(|x| x.path != p_str) // fix bug?
                            .map(|x| {
                                (
                                    PathBuf::from(x.path.clone())
                                        .parent()
                                        .map(|x| x.to_path_buf()),
                                    {
                                        let rel_path = PathBuf::from(
                                            x.path.split('/').last().unwrap_or(&x.path),
                                        );
                                        FsNode {
                                            path: rel_path,
                                            attr: Self::to_attr(&x),
                                        }
                                    },
                                )
                            })
                            .into_iter()
                        {
                            if let Some(kp) = k {
                                map.entry(kp).or_insert_with(Vec::new).push(v);
                            }
                        }
                        if map.is_empty() {
                            self.cache_lookup.insert(path.to_path_buf(), vec![]);
                            ret = Some(vec![]);
                        } else {
                            for (k, v) in map {
                                if k == path {
                                    ret = Some(v.clone());
                                }
                                self.cache_lookup.insert(k, v);
                            }
                        }
                        ret
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }?;
        for x in ret {
            output.push(x);
        }

        Some(output)
    }

    pub fn do_dump(&self, path: &Path, offset: u64, size: usize) -> Option<Arc<Vec<u8>>> {
        match self.cache_dump.get(path) {
            Some(ret) => Some(ret),
            None => {
                let ret = {
                    let m_api = &*self.api.read().unwrap();
                    if let Some(api) = m_api {
                        self.rt.block_on(async move {
                            api.dump_file(path.to_str().unwrap(), Some(offset), if size >= usize::MAX { None } else { Some(size as usize) })
                                .await
                                .ok()
                        })
                    } else {
                        None
                    }
                    .map(|x| BASE64_STANDARD.decode(x).ok())
                    .flatten()
                };
                if let Some(r) = ret {
                    let arc = Arc::new(r);
                    self.cache_dump.insert(path.to_path_buf(), arc.clone());
                    Some(arc)
                } else {
                    None
                }
            }
        }
    }

    pub fn do_write(
        &self,
        path: &Path,
        new_path: Option<&Path>,
        offset: Option<u64>,
        data: Option<&[u8]>,
        atime: Option<SystemTime>,
        mtime: Option<SystemTime>,
        t_len: Option<u64>,
    ) -> Option<()> {
        {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                self.rt.block_on(async move {
                    api.put_file(PutForFileApi {
                        path: path.to_str().unwrap().to_owned(),
                        base64: data.map(|x| BASE64_STANDARD.encode(x)),
                        new_path: new_path.map(|x| x.to_str().unwrap().to_owned()),
                        atime: atime.map(|x| to_dto_time!(x)),
                        mtime: mtime.map(|x| to_dto_time!(x)),
                        offset: offset,
                        t_len: t_len,
                    })
                    .await
                    .ok()
                })
            } else {
                None
            }
        }?;
        self.delete_cache(path);
        if let Some(np) = new_path {
            self.delete_cache(np);
        }
        Some(())
    }

    pub fn do_create(&self, path: &Path, is_dir: bool) -> Option<()> {
        {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                self.rt.block_on(async move {
                    api.post_file(PostForFileApi {
                        path: path.to_str().unwrap().to_owned(),
                        base64: None,
                        offset: None,
                        is_dir,
                    })
                    .await
                    .ok()
                })
            } else {
                None
            }
        }?;
        self.delete_cache(path);
        Some(())
    }

    pub fn do_delete(&self, path: &Path) -> Option<()> {
        {
            let m_api = &*self.api.read().unwrap();
            if let Some(api) = m_api {
                self.rt
                    .block_on(async move { api.delete_file(path.to_str().unwrap()).await.ok() })
            } else {
                None
            }
        }?;
        self.delete_cache(path);
        Some(())
    }
}

#[cfg(unix)]
use fuse_mt::{
    CallbackResult, CreatedEntry, DirectoryEntry, FileAttr, FileType, FilesystemMT, RequestInfo,
    ResultCreate, ResultEmpty, ResultEntry, ResultOpen, ResultReaddir, ResultSlice, ResultWrite,
};

#[cfg(unix)]
impl From<GenericFileType> for FileType {
    fn from(value: GenericFileType) -> Self {
        match value {
            GenericFileType::BlockDevice => Self::BlockDevice,
            GenericFileType::CharDevice => Self::CharDevice,
            GenericFileType::Directory => Self::Directory,
            GenericFileType::NamedPipe => Self::NamedPipe,
            GenericFileType::RegularFile => Self::RegularFile,
            GenericFileType::Socket => Self::Socket,
            GenericFileType::Symlink => Self::Symlink,
        }
    }
}

#[cfg(unix)]
impl From<GenericFileAttr> for FileAttr {
    fn from(value: GenericFileAttr) -> Self {
        FileAttr {
            size: value.size,
            blocks: value.blocks,
            atime: value.atime,
            mtime: value.mtime,
            ctime: value.ctime,
            crtime: value.crtime,
            kind: value.kind.into(),
            perm: value.perm,
            nlink: value.nlink,
            uid: value.uid,
            gid: value.gid,
            rdev: value.rdev,
            flags: value.flags,
        }
    }
}

#[cfg(unix)]
impl FilesystemMT for NeptisFS {
    fn readdir(&self, _req: RequestInfo, path: &Path, _fh: u64) -> ResultReaddir {
        // Attempt to read the entire directory.
        let ret = self
            .do_readdir(path)
            .map(|y| {
                y.into_iter()
                    .map(|x| DirectoryEntry {
                        name: x.path.into_os_string(),
                        kind: x.attr.kind.into(),
                    })
                    .collect::<Vec<_>>()
            })
            .ok_or(libc::ENETUNREACH)?;
        Ok(ret)
    }

    fn getattr(&self, _req: RequestInfo, path: &Path, _fh: Option<u64>) -> ResultEntry {
        self.do_find(path).map(|x| (FS_DURATION, x.attr.into()))
    }

    fn read(
        &self,
        _req: RequestInfo,
        path: &Path,
        _fh: u64,
        offset: u64,
        size: u32,
        callback: impl FnOnce(ResultSlice<'_>) -> CallbackResult,
    ) -> CallbackResult {
        if let Some(full_data) = self.do_dump(path, 0, usize::MAX) {
            let start = offset as usize;
            let end = (start + size as usize).min(full_data.len());
            if start >= full_data.len() {
                return callback(Ok(&[]));
            }
            let slice = &full_data[start..end];
            callback(Ok(slice))
        } else {
            callback(Err(libc::ENETDOWN))
        }
    }

    fn open(&self, _req: RequestInfo, _path: &Path, _flags: u32) -> ResultOpen {
        Ok((42, _flags))
    }

    fn opendir(&self, _req: RequestInfo, _path: &Path, _flags: u32) -> ResultOpen {
        Ok((42, _flags))
    }

    fn release(
        &self,
        _req: RequestInfo,
        _path: &Path,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> ResultEmpty {
        Ok(())
    }

    fn releasedir(&self, _req: RequestInfo, _path: &Path, _fh: u64, _flags: u32) -> ResultEmpty {
        Ok(())
    }

    fn access(&self, _req: RequestInfo, _path: &Path, _mask: u32) -> ResultEmpty {
        Ok(())
    }

    fn write(
        &self,
        _req: RequestInfo,
        path: &Path,
        _fh: u64,
        offset: u64,
        data: Vec<u8>,
        _flags: u32,
    ) -> ResultWrite {
        self.do_write(
            path,
            None,
            Some(offset),
            Some(data.as_slice()),
            None,
            None,
            None,
        )
        .map(|_| data.len() as u32)
        .ok_or(libc::ENETUNREACH)
    }

    fn create(
        &self,
        _req: RequestInfo,
        parent: &Path,
        name: &OsStr,
        _mode: u32,
        flags: u32,
    ) -> ResultCreate {
        let path = parent.join(name);
        self.do_create(&path, false).ok_or(libc::ENETUNREACH)?;
        self.do_find(&path).map(|x| CreatedEntry {
            ttl: FS_DURATION,
            attr: x.attr.into(),
            fh: 42,
            flags,
        })
    }

    fn fsync(&self, _req: RequestInfo, _path: &Path, _fh: u64, _datasync: bool) -> ResultEmpty {
        Ok(())
    }

    fn fsyncdir(&self, _req: RequestInfo, _path: &Path, _fh: u64, _datasync: bool) -> ResultEmpty {
        Ok(())
    }

    fn utimens(
        &self,
        _req: RequestInfo,
        path: &Path,
        _fh: Option<u64>,
        atime: Option<std::time::SystemTime>,
        mtime: Option<std::time::SystemTime>,
    ) -> ResultEmpty {
        self.do_write(path, None, None, None, atime, mtime, None)
            .ok_or(libc::ENETUNREACH)
    }

    fn unlink(&self, _req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        self.do_delete(&parent.join(name)).ok_or(libc::ENETUNREACH)
    }

    fn rename(
        &self,
        _req: RequestInfo,
        parent: &Path,
        name: &OsStr,
        newparent: &Path,
        newname: &OsStr,
    ) -> ResultEmpty {
        self.do_write(
            &parent.join(name),
            Some(&newparent.join(newname)),
            None,
            None,
            None,
            None,
            None,
        )
        .ok_or(libc::ENETUNREACH)
    }

    fn truncate(&self, _req: RequestInfo, path: &Path, _fh: Option<u64>, size: u64) -> ResultEmpty {
        self.do_write(path, None, None, None, None, None, Some(size))
            .ok_or(libc::ENETUNREACH)
    }

    fn mkdir(&self, _req: RequestInfo, parent: &Path, name: &OsStr, _mode: u32) -> ResultEntry {
        let path = parent.join(name);
        self.do_create(&path, true).ok_or(libc::ENETUNREACH)?;
        self.do_find(&path).map(|x| (FS_DURATION, x.attr.into()))
    }

    fn rmdir(&self, _req: RequestInfo, parent: &Path, name: &OsStr) -> ResultEmpty {
        self.do_delete(&parent.join(name)).ok_or(libc::ENETUNREACH)
    }
}
