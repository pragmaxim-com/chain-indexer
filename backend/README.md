### Building OpenAPI

First we build our `openapi.json` specification which will later be served at http://127.0.0.1:8082/swagger :

```
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
nvm install 20
npm install --global yarn
npm install --global @redocly/cli@latest

yarn bundle
```
