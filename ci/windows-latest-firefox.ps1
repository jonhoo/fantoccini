choco install firefox
Invoke-WebRequest -Uri "https://github.com/mozilla/geckodriver/releases/download/v0.30.0/geckodriver-v0.30.0-win64.zip" -OutFile geckodriver.zip
Expand-Archive -LiteralPath geckodriver.zip -DestinationPath .
Start-Process -FilePath geckodriver
Start-Sleep -Seconds 1
