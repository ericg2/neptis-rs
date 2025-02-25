@ECHO OFF
REM Updates the file with the latest OpenAPI JSON specification
REM Copyright (c) 2025 Eric (ericg2) Gold
REM Licensed under terms of GPLv3
@ECHO ON

RD /S /Q temp
MKDIR temp
start "OpenAPI Generator" cmd /k "npx @openapitools/openapi-generator-cli generate -i openapi.json -g rust -o temp && exit"
TIMEOUT /T 15
RD /S /Q src
MKDIR src
XCOPY /E /I "temp\src" src
RD /S /Q temp
clear
git add .
git commit -m "API Update"
TIMEOUT /T 5
git push
	