@echo off
setlocal

set "CONFIG=%~1"
if "%CONFIG%"=="" set "CONFIG=Final Release|Win32"

set "ROOT=%~dp0.."
pushd "%ROOT%" || exit /b 1

set "DEVENV=%VS71COMNTOOLS%..\IDE\devenv.com"
if exist "%DEVENV%" goto :have_devenv

for %%D in (
  "%ProgramFiles(x86)%\Microsoft Visual Studio .NET 2003\Common7\IDE\devenv.com"
  "%ProgramFiles%\Microsoft Visual Studio .NET 2003\Common7\IDE\devenv.com"
) do (
  if exist "%%~D" (
    set "DEVENV=%%~D"
    goto :have_devenv
  )
)

echo Could not find Visual Studio .NET 2003 devenv.com.
echo Install VS2003 on the self-hosted runner or set VS71COMNTOOLS.
popd
exit /b 1

:have_devenv
if not exist "Boost-1.32.0\libs\boost_python-vc71-mt-1_32.lib" (
  echo Missing Boost.Python VC7.1 library.
  popd
  exit /b 1
)

if not exist "Python24\libs\python24.lib" (
  echo Missing Python 2.4 import library.
  popd
  exit /b 1
)

if not exist "..\Beyond the Sword\Assets" mkdir "..\Beyond the Sword\Assets"
if not exist "artifacts" mkdir "artifacts"

echo Building CvGameCoreDLL.vcproj %CONFIG%
"%DEVENV%" "CvGameCoreDLL.vcproj" /build "%CONFIG%"
if errorlevel 1 (
  popd
  exit /b 1
)

if not exist "..\Beyond the Sword\Assets\CvGameCoreDLL.dll" (
  echo Build completed but CvGameCoreDLL.dll was not found.
  popd
  exit /b 1
)

copy /y "..\Beyond the Sword\Assets\CvGameCoreDLL.dll" "artifacts\CvGameCoreDLL.dll" > nul

popd
endlocal
