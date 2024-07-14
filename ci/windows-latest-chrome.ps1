npm i puppeteer
# pin to 125 due to https://issues.chromium.org/issues/42323434
npx @puppeteer/browsers install chrome@125
npx @puppeteer/browsers install chromedriver@125
Start-Process -FilePath chromedriver
Start-Sleep -Seconds 1
