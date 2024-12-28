# Order Assistant

Takes a menu, and writes an agent to do order communication with someone.

### Run Dev

``` sh
cp env.sample .env # and fill in required details
docker-compose -f docker-compose.dev.yml up
cargo run
```

### Run Locally

Fill in the environment parameter with the sample from `env.sample`

``` sh
export OPENAI_API_KEY=key
export API_KEYS=key1,key2,key3
docker-compose up
```

### Generate Docs

``` sh
cargo doc --open
```

### Running Tests
_TODO_

### Running E2E Tests
_TODO_
