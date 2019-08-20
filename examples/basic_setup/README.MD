## Setup

This example assumes you have geckodriver or chromedriver listening at port 4444.

You can start the webdriver instance by: 

### geckodriver
`geckodriver --port 4444`

### chromedriver
`chromedriver --port=4444`

The above commands assume either driver is on your path otherwise, provide the entire path
to the binary: 

`path/to/webdriver/geckodriver -p 4444`

## To Run

Call `cargo run` from the root of the `basic_setup` directory.
