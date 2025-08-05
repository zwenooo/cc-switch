@echo off
echo Starting CC Switch Development Server...
echo.

echo [1/2] Starting Vite dev server...
start /B cmd /c "pnpm run dev:renderer"

echo [2/2] Waiting for Vite to start...
timeout /t 5 /nobreak > nul

echo Starting Electron...
pnpm run dev:electron

echo.
echo Development server started!