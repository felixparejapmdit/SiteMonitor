@echo off
setlocal
python "%~dp0deploy-to-nas.py" %*
set "deploy_exit=%errorlevel%"
if "%~1"=="" pause
exit /b %deploy_exit%
