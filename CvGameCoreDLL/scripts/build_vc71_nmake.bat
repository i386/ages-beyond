@echo off
setlocal

set "TOOLCHAIN_ROOT=%~1"
if "%TOOLCHAIN_ROOT%"=="" set "TOOLCHAIN_ROOT=%VC71_ROOT%"
if "%TOOLCHAIN_ROOT%"=="" set "TOOLCHAIN_ROOT=%RUNNER_TEMP%\vc71"

set "ROOT=%~dp0.."
pushd "%ROOT%" || exit /b 1

if not exist "%TOOLCHAIN_ROOT%" (
  echo VC7.1 toolchain root not found: %TOOLCHAIN_ROOT%
  popd
  exit /b 1
)

set "VCROOT="
for %%D in ("%TOOLCHAIN_ROOT%\VC7" "%TOOLCHAIN_ROOT%\VC" "%TOOLCHAIN_ROOT%") do (
  if exist "%%~D\bin\cl.exe" set "VCROOT=%%~D"
)

if "%VCROOT%"=="" (
  for /f "delims=" %%F in ('dir /b /s "%TOOLCHAIN_ROOT%\cl.exe" 2^>nul') do (
    if "%%~nxF"=="cl.exe" set "VCROOT=%%~dpF.."
  )
)

if "%VCROOT%"=="" (
  echo Could not find cl.exe below %TOOLCHAIN_ROOT%.
  popd
  exit /b 1
)

set "COMMON7ROOT="
if exist "%TOOLCHAIN_ROOT%\Common7" set "COMMON7ROOT=%TOOLCHAIN_ROOT%\Common7"

set "SDKINCLUDE="
for %%D in ("%TOOLCHAIN_ROOT%\PlatformSDK" "%TOOLCHAIN_ROOT%\SDK" "%TOOLCHAIN_ROOT%\Microsoft Platform SDK" "%TOOLCHAIN_ROOT%") do (
  if exist "%%~D\include\windows.h" set "SDKINCLUDE=%%~D\include"
)
if "%SDKINCLUDE%"=="" (
  for /f "delims=" %%F in ('dir /b /s "%TOOLCHAIN_ROOT%\windows.h" 2^>nul') do (
    if "%%~nxF"=="windows.h" set "SDKINCLUDE=%%~dpF"
  )
)

if "%SDKINCLUDE%"=="" (
  echo Could not find Platform SDK include directory containing windows.h.
  popd
  exit /b 1
)

set "SDKROOT=%SDKINCLUDE%\.."
for %%D in ("%SDKROOT%") do set "SDKROOT=%%~fD"

set "NMAKEPATH="
for /f "delims=" %%F in ('dir /b /s "%ProgramFiles%\Microsoft Visual Studio\2022\nmake.exe" "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\nmake.exe" 2^>nul') do set "NMAKEPATH=%%~dpF"

set "PATH=%VCROOT%\bin;%COMMON7ROOT%\IDE;%COMMON7ROOT%\Tools;%SDKROOT%\bin;%NMAKEPATH%;%PATH%"
set "INCLUDE=%CD%\Boost-1.32.0\include;%CD%\Python24\include;%VCROOT%\include;%SDKINCLUDE%;%INCLUDE%"
set "LIB=%CD%\Boost-1.32.0\libs;%CD%\Python24\libs;%VCROOT%\lib;%SDKROOT%\lib;%LIB%"

where cl.exe || goto :missing_tool
where link.exe || goto :missing_tool
where nmake.exe || goto :missing_tool
where rc.exe || goto :missing_tool
goto :have_tools

:missing_tool
  echo Required VC7.1 build tools are missing.
  popd
  exit /b 1

:have_tools

nmake /nologo /f Makefile.vc71 CFG=FinalRelease
set "BUILD_EXIT=%ERRORLEVEL%"

popd
exit /b %BUILD_EXIT%
