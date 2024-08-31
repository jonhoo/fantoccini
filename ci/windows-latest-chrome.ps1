npm i puppeteer
npx @puppeteer/browsers install chrome@stable
npx @puppeteer/browsers install chromedriver@stable
Start-Process -FilePath chromedriver -ArgumentList "--port=9515"
Start-Sleep -Seconds 1
