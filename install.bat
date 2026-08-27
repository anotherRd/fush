@echo off
setlocal

cd /d "%~dp0"

:: elevated installation mode
if /I "%~1"=="__install" goto install

echo INSTALLING

:: build as normal user
cargo build --release
if errorlevel 1 exit /b %errorlevel%

:: check whether run as administrator
fltmc >nul 2>&1
if errorlevel 1 (
    echo Requesting Administrator privileges...

    powershell -NoProfile -Command ^
        "Start-Process '%~f0' -Verb RunAs -ArgumentList '__install' -WorkingDirectory '%CD%'"

    exit /b
)

:install

set "INSTALL_DIR=C:\Program Files\fush"

if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
    if errorlevel 1 exit /b %errorlevel%
)

copy /Y "%~dp0target\release\fush.exe" "%INSTALL_DIR%\fush.exe"
if errorlevel 1 exit /b %errorlevel%

:: add installation directory to machine PATH
powershell -NoProfile -Command ^
    "$dir = '%INSTALL_DIR%'; $path = [Environment]::GetEnvironmentVariable('Path', 'Machine'); if (($path -split ';') -notcontains $dir) { [Environment]::SetEnvironmentVariable('Path', $path.TrimEnd(';') + ';' + $dir, 'Machine') }"

if errorlevel 1 exit /b %errorlevel%

echo.
echo Successfully installed to %INSTALL_DIR%\fush.exe
echo Added %INSTALL_DIR% to machine PATH.

endlocal