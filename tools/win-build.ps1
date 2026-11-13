# Local Windows build environment for this repo's probes and samples.
# Usage: . tools\win-build.ps1; cargo test --release ...
# Sets Div's CI Skia profile (also in .cargo/config.toml), the VS 2022 Build
# Tools environment (VS 2026 lacks the VC libraries) and a short target dir.
$env:Path = "$env:USERPROFILE\.cargo\bin;C:\Program Files\LLVM\bin;C:\Program Files\CMake\bin;$env:LOCALAPPDATA\Microsoft\WinGet\Links;" + $env:Path
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
$env:PYTHONUTF8 = "1"
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = "E:\open-ui\out\cargo-sample-canvas" }
$vcvars = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (Test-Path $vcvars) {
    cmd /c "`"$vcvars`" >nul && set" | ForEach-Object {
        if ($_ -match '^(LIB|LIBPATH|INCLUDE|Path)=(.*)$') { Set-Item "env:$($Matches[1])" $Matches[2] }
    }
    $env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
}
