@echo off
rem 一键构建 RA3 地图（cmd 包装，不受 PowerShell 执行策略限制）
rem   build.cmd                        默认配方
rem   build.cmd --map <id> --install
rem   build.cmd --only map --only verify
setlocal
set "PY=%USERPROFILE%\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\python\python.exe"
if not exist "%PY%" set "PY=python"
"%PY%" -X utf8 "%~dp0tools\build_all.py" %*
exit /b %ERRORLEVEL%
