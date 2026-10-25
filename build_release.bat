@echo off
setlocal
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

echo ===================================================
echo   YMPP Comics Library - Release Build Starting
echo ===================================================

echo [1/3] Building Vue 3 Frontend...
call npm run build
if %errorlevel% neq 0 (
    echo [ERROR] Frontend build failed.
    exit /b %errorlevel%
)

echo [2/3] Compiling Tauri Rust Release Binary...
call npx tauri build --no-bundle
if %errorlevel% neq 0 (
    echo [ERROR] Tauri build failed.
    exit /b %errorlevel%
)

echo [3/3] Organizing Portable Release Package...
if not exist "release" mkdir "release"

if exist "src-tauri\target\release\ympp-comics-library.exe" (
    if exist "release\YMPPComicsLibrary.exe.old" del /f /q "release\YMPPComicsLibrary.exe.old" > nul 2>nul
    copy /y "src-tauri\target\release\ympp-comics-library.exe" "release\YMPPComicsLibrary.exe" > nul 2>nul
    if errorlevel 1 (
        move /y "release\YMPPComicsLibrary.exe" "release\YMPPComicsLibrary.exe.old" > nul 2>nul
        copy /y "src-tauri\target\release\ympp-comics-library.exe" "release\YMPPComicsLibrary.exe" > nul
    )
    echo  - Portable Executable copied to: release\YMPPComicsLibrary.exe
)

echo ===================================================
echo   YMPP Comics Library - Release Build SUCCESS!
echo   Executable Location: %~dp0release\YMPPComicsLibrary.exe
echo ===================================================

endlocal
