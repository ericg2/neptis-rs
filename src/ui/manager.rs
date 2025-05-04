use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use inquire::{
    Confirm, MultiSelect, Select,
    list_option::ListOption,
    required,
    validator::{self, Validation},
};
use tokio::runtime::Runtime;

use crate::{
    UiApp,
    apis::{NeptisError, api::WebApi},
};

// pub type PromptType<T> = fn(&mut T);
// pub type PropGetType<T> = fn(&T) -> String;
// pub type ModifyType<T> = fn(Vec<T>, &T) -> Result<(), NeptisError>;
// pub type PullType<T> = fn() -> Result<Vec<T>, NeptisError>;
// pub type DeleteType<T> = fn(&T) -> Result<(), NeptisError>;

// pub type PromptType<T> = Box<dyn FnMut(&mut T)>;
// pub type PropGetType<T> = Box<dyn FnMut(&T) -> String>;
// pub type ModifyType<T> = Box<dyn FnMut(Vec<T>, &T) -> Result<(), NeptisError>>;
// pub type PullType<T> = Box<dyn FnMut() -> Result<Vec<T>, NeptisError>>;
// pub type DeleteType<T> = Box<dyn FnMut(&T) -> Result<(), NeptisError>>;

pub struct ModelProperty<T> {
    name: String,
    is_pk: bool,
    f_prompt: PromptType<T>,
    f_get: PropGetType<T>,
    for_create: bool,
    for_update: bool,
}

impl<T: Clone + ToShortIdString + Default> ModelProperty<T> {
    fn _new(
        name: impl Into<String>,
        is_pk: bool,
        f_prompt: PromptType<T>,
        f_get: PropGetType<T>,
        for_create: bool,
        for_update: bool,
    ) -> Self {
        Self {
            is_pk,
            name: name.into(),
            f_prompt: f_prompt.into(),
            f_get: f_get.into(),
            for_create,
            for_update,
        }
    }

    pub fn new(
        name: impl Into<String>,
        is_pk: bool,
        f_prompt: PromptType<T>,
        f_get: PropGetType<T>,
    ) -> Self {
        Self::_new(name, is_pk, f_prompt, f_get, true, true)
    }

    pub fn new_for_update_only(
        name: impl Into<String>,
        is_pk: bool,
        f_prompt: PromptType<T>,
        f_get: PropGetType<T>,
    ) -> Self {
        Self::_new(name, is_pk, f_prompt, f_get, false, true)
    }

    pub fn new_for_create_only(
        name: impl Into<String>,
        is_pk: bool,
        f_prompt: PromptType<T>,
        f_get: PropGetType<T>,
    ) -> Self {
        Self::_new(name, is_pk, f_prompt, f_get, true, false)
    }
}

pub struct ModelExtraOption<'a, T> {
    name: String,
    callback: Box<dyn Fn(&T) + 'a>,
}

impl<'a, T: Clone + ToShortIdString + Default> ModelExtraOption<'a, T> {
    pub fn new(name: impl Into<String>, callback: impl Fn(&T) + 'a) -> Self {
        ModelExtraOption {
            name: name.into(),
            callback: Box::new(callback),
        }
    }

    pub fn call(&self, value: &T) {
        (self.callback)(value);
    }
}

pub trait ToShortIdString {
    fn to_short_id_string(&self) -> String;
}

// pub type PromptType<T> = Box<dyn FnMut(&mut T)>;
// pub type PropGetType<T> = Box<dyn FnMut(&T) -> String>;
// pub type ModifyType<T> = Box<dyn FnMut(Vec<T>, &T) -> Result<(), NeptisError>>;
// pub type PullType<T> = Box<dyn FnMut() -> Result<Vec<T>, NeptisError>>;
// pub type DeleteType<T> = Box<dyn FnMut(&T) -> Result<(), NeptisError>>;
pub struct ApiContext<'a> {
    pub rt: Runtime,
    pub api: Option<&'a WebApi>,
}

pub type PromptType<T> = fn(&mut T);
pub type PropGetType<T> = fn(&T) -> String;
pub type ModifyType<T> = Box<dyn FnMut(&mut ApiContext, Vec<T>, &T) -> Result<(), NeptisError>>;
pub type PullType<T> = Box<dyn FnMut(&mut ApiContext) -> Result<Vec<T>, NeptisError>>;
pub type DeleteType<T> = Box<dyn FnMut(&mut ApiContext, &T) -> Result<(), NeptisError>>;

pub struct ModelManager<'a, T> {
    properties: Vec<ModelProperty<T>>,
    options: Vec<ModelExtraOption<'a, T>>,
    allow_back: bool,
    func_update_item: Option<ModifyType<T>>,
    func_delete_item: Option<DeleteType<T>>,
    func_pull_items: PullType<T>,
    str_select: String,
    str_create: String,
    str_edit: String,
    str_delete: String,
    str_select_title: String,
    str_create_title: String,
    str_modify_title: String,
    ctx: ApiContext<'a>,
}

impl<'a, T: Clone + ToShortIdString + Default> ModelManager<'a, T> {
    pub fn new(
        api: Option<&'a WebApi>,
        properties: Vec<ModelProperty<T>>,
        func_pull_items: PullType<T>,
    ) -> Self {
        let ctx = ApiContext {
            api,
            rt: Runtime::new().unwrap(),
        };
        Self {
            ctx,
            properties,
            options: vec![],
            allow_back: false,
            func_update_item: None,
            func_delete_item: None,
            str_select: "Select".into(),
            str_create: "Create".into(),
            str_edit: "Edit".into(),
            str_delete: "Delete".into(),
            str_select_title: "Please select an item".into(),
            str_create_title: "".into(),
            str_modify_title: "".into(),
            func_pull_items: func_pull_items.into(),
        }
    }
    pub fn add(
        mut self,
        name: impl Into<String>,
        f_prompt: PromptType<T>,
        f_get: PropGetType<T>,
        is_pk: bool,
        for_create: bool,
        for_update: bool,
    ) -> Self {
        self.properties.push(ModelProperty {
            name: name.into(),
            f_prompt,
            f_get,
            is_pk,
            for_create,
            for_update,
        });
        self
    }

    pub fn with_modify(mut self, func: ModifyType<T>) -> ModelManager<'a, T> {
        self.func_update_item = Some(func.into());
        self
    }
    pub fn with_delete(mut self, func: DeleteType<T>) -> ModelManager<'a, T> {
        self.func_delete_item = Some(func.into());
        self
    }
    // pub fn with_options(mut self, options: impl Into<Vec<ModelExtraOption<'a, T>>>) -> Self {
    //     self.options = options.into();
    //     self
    // }
    pub fn can_modify(&self) -> bool {
        self.func_update_item.is_some()
    }
    pub fn with_back(mut self) -> ModelManager<'a, T> {
        self.allow_back = true;
        self
    }

    pub fn can_delete(&self) -> bool {
        self.func_delete_item.is_some()
    }

    pub fn with_select_title(mut self, title: impl Into<String>) -> Self {
        self.str_select_title = title.into();
        self
    }

    pub fn with_modify_title(mut self, title: impl Into<String>) -> Self {
        self.str_modify_title = title.into();
        self
    }

    pub fn with_create_title(mut self, title: impl Into<String>) -> Self {
        self.str_create_title = title.into();
        self
    }

    fn show_manage_item(&mut self, item: Option<T>, multi: bool) -> Result<Vec<T>, NeptisError> {
        let allow_pk = item.is_none();
        let mut use_item = item.clone().unwrap_or_default();
        // Iterate through each element to begin managing.
        loop {
            clearscreen::clear().unwrap();
            println!("Neptis Management");
            println!("You will be asked to confirm all this information.");
            println!(
                "{}\n",
                if allow_pk {
                    self.str_create_title.as_str()
                } else {
                    self.str_modify_title.as_str()
                }
            );
            for prop in self.properties.iter_mut() {
                if allow_pk && !prop.for_create {
                    continue; // if creating and not for create
                }
                if !allow_pk && !prop.for_update {
                    continue; // if updating and not for update
                }
                if !prop.is_pk || allow_pk {
                    (&mut prop.f_prompt)(&mut use_item);
                }
            }

            // Make sure there are no primary key issues
            let mut error = false;
            if allow_pk {
                // Phase 1: Collect all necessary immutable data first
                let items = (&mut self.func_pull_items)(&mut self.ctx)?;
                let pk_indices: Vec<usize> = self
                    .properties
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.is_pk)
                    .map(|(i, _)| i)
                    .collect();

                // Phase 2: Process items with mutable access
                for i in items {
                    if error {
                        break;
                    }

                    for &idx in &pk_indices {
                        // Split borrow - get mutable access to just this property
                        let (_, tail) = self.properties.split_at_mut(idx);
                        let prop = &mut tail[0];

                        // Call the FnMut closure
                        let val = (&mut prop.f_get)(&i);

                        // Temporarily get the property name (shared reference)
                        let prop_name = prop.name.to_string();

                        // Phase 3: Immutable comparison
                        let p_val = self
                            .properties
                            .iter_mut()
                            .find(|x| x.name.clone() == prop_name)
                            .map(|x| (&mut x.f_get)(&use_item));

                        if let Some(p_val) = p_val {
                            if p_val == val {
                                error = true;
                                break;
                            }
                        }
                    }
                }

                if error {
                    println!("\n> Primary key validation error - element exists!");
                    thread::sleep(Duration::from_secs(3));
                    continue;
                }
            }

            clearscreen::clear().unwrap();
            println!("Please confirm the information below:");
            println!(
                "{}",
                self.properties
                    .iter_mut()
                    .map(|x| format!("> {} -> '{}'", x.name, (&mut x.f_get)(&use_item)))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            match Select::new(
                "Are you sure this is correct?",
                vec!["Go Back", "No", "Yes", "Reset"],
            )
            .prompt()
            .expect("Failed to show prompt!")
            {
                "Yes" => break,
                "Reset" => {
                    if let Some(i) = item.clone() {
                        use_item = i;
                    } else {
                        use_item = T::default();
                    }
                    continue;
                }
                "Go Back" => return self.do_raw_display(multi),
                _ => continue,
            }
        }
        // Do the delete/add and proceed.
        if let Some(ref mut func) = self.func_update_item {
            let all_items = (self.func_pull_items)(&mut self.ctx)?;
            (func)(&mut self.ctx, all_items, &use_item)?;
        }

        return self.do_raw_display(multi);
    }
    fn show_delete_items(&mut self, items: Vec<T>, multi: bool) -> Result<Vec<T>, NeptisError> {
        clearscreen::clear().unwrap();
        println!("Please confirm you would like to delete the following:");
        for (i, item) in items.iter().enumerate() {
            println!(
                "{}. {}\n",
                i,
                self.properties
                    .iter_mut()
                    .map(|x| format!("> {} -> '{}'", x.name, (&mut x.f_get)(&item)))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }

        if Confirm::new("This is a destructive action. Are you sure?")
            .prompt()
            .expect("Failed to show prompt!")
        {
            if let Some(ref mut func) = self.func_delete_item {
                for item in items {
                    (func)(&mut self.ctx, &item)?;
                }
            }
        }
        return self.do_raw_display(multi);
    }

    fn do_raw_display(&mut self, multi: bool) -> Result<Vec<T>, NeptisError> {
        clearscreen::clear().unwrap();
        let all_items = (self.func_pull_items)(&mut self.ctx)?;
        let id_sel = {
            let mut s_items = vec![];
            if self.allow_back {
                s_items.push("Go Back".to_string());
            }
            s_items.extend(
                all_items
                    .iter()
                    .map(|x| x.to_short_id_string())
                    .collect::<Vec<_>>(),
            );
            if self.func_update_item.is_some() {
                s_items.push(self.str_create.clone());
            }
            if multi {
                MultiSelect::new(self.str_select_title.as_str(), s_items)
                    .with_page_size(30)
                    // .with_validator(|selection: &[ListOption<&String>]| {
                    //     if selection.is_empty() {
                    //         return Ok(Validation::Invalid(
                    //             "You must select at least one option.".into(),
                    //         ));
                    //     }
                    //     Ok(Validation::Valid)
                    // })
                    .prompt()
                    .expect("Failed to show prompt!")
            } else {
                vec![
                    Select::new(self.str_select_title.as_str(), s_items)
                        .with_page_size(30)
                        .prompt()
                        .expect("Failed to show prompt!"),
                ]
            }
        };

        if id_sel.iter().any(|x| *x == self.str_create) && self.func_update_item.is_some() {
            self.show_manage_item(None, multi)
        } else {
            if self.allow_back && id_sel.len() == 0 || id_sel.iter().any(|x| *x == "Go Back") {
                return Ok(vec![]);
            }
            // For each item, attempt to validate to ensure they exist.
            let action_title = id_sel.join("\n");
            let f_items = id_sel
                .into_iter()
                .map(|x| {
                    all_items
                        .iter()
                        .find(|y| y.to_short_id_string() == x)
                        .expect("Item does not exist!")
                        .clone()
                })
                .collect::<Vec<_>>();

            // If the only option is to select, then just do it.
            if self.func_delete_item.is_none()
                && self.func_update_item.is_none()
                && self.options.len() <= 0
            {
                return Ok(f_items);
            }
            let action_sel = Select::new(format!("Action for '{}'", action_title).as_str(), {
                let mut all_actions = vec![self.str_select.clone()];
                all_actions.extend(self.options.iter().map(|x| x.name.clone()));
                if self.func_update_item.is_some() && f_items.len() == 1 {
                    all_actions.push(self.str_edit.clone()); // cannot modify > 1 items!
                }
                if self.func_delete_item.is_some() {
                    all_actions.push(self.str_delete.clone());
                }
                all_actions.push("Go Back".to_string());
                all_actions
            })
            .prompt()
            .expect("Failed to display prompt!");
            if action_sel == self.str_select {
                Ok(f_items)
            } else if action_sel == self.str_edit && self.func_update_item.is_some() {
                self.show_manage_item(
                    Some(f_items.first().expect("Failed to find first item!").clone()),
                    multi,
                )
            } else if action_sel == self.str_delete && self.func_delete_item.is_some() {
                self.show_delete_items(f_items, multi)
            } else {
                return self.do_raw_display(multi);
            }
        }
    }

    pub fn do_display(&mut self) -> Result<Option<T>, NeptisError> {
        self.do_raw_display(false).map(|x| x.first().cloned())
    }

    pub fn do_multi_display(&mut self) -> Result<Vec<T>, NeptisError> {
        self.do_raw_display(true)
    }
}
