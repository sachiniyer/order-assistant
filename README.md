# Order Assistant

Takes a menu, and writes an agent to do order communication with someone.

### Development

``` sh
cp env.sample .env # and fill in required details
docker-compose -f docker-compose.dev.yml up
cargo run
```

### Run Locally

Fill in the environment parameter with the sample from `env.sample`

``` yaml
    environment:
```

``` sh
docker-compose up
```
