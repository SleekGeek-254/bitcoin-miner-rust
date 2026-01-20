@echo off
echo === CLEANING AND SETTING UP ENVIRONMENT ===
echo.

echo 1. Cleaning duplicate PATH entries...
set PATH=D:\AppData\NVIDIA GPU Computing Toolkit\CUDA\v12.8\bin;C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.41.34120\bin\Hostx64\x64;C:\WINDOWS\system32;C:\WINDOWS;C:\WINDOWS\System32\Wbem;C:\WINDOWS\System32\WindowsPowerShell\v1.0\

echo 2. Setting Visual Studio variables...
set VCINSTALLDIR=C:\Program Files\Microsoft Visual Studio\2022\Community\VC
set VCToolsInstallDir=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.41.34120
set WindowsSdkDir=C:\Program Files (x86)\Windows Kits\10

echo 3. Setting CUDA to v12.8...
set CUDA_PATH=D:\AppData\NVIDIA GPU Computing Toolkit\CUDA\v12.8

echo 4. Setting INCLUDE and LIB...
set INCLUDE=%VCToolsInstallDir%\include;%CUDA_PATH%\include
set LIB=%VCToolsInstallDir%\lib\x64;%CUDA_PATH%\lib\x64

echo.
echo === COMPILING CUDA KERNEL ===
echo.

echo 5. Checking nvcc...
where nvcc

echo 6. Compiling mining_kernel.cu...
nvcc -ptx -arch=sm_75 -o kernels/mining_kernel.ptx kernels/mining_kernel.cu

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ✅ SUCCESS! CUDA kernel compiled.
    echo.
) else (
    echo.
    echo ❌ FAILED to compile CUDA kernel.
    echo.
)

echo 7. Checking if PTX file was created...
if exist kernels/mining_kernel.ptx (
    echo ✅ PTX file created: kernels/mining_kernel.ptx
) else (
    echo ❌ PTX file NOT created
)

pause