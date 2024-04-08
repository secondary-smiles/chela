[![Build and Push Docker Image](https://github.com/secondary-smiles/chela/actions/workflows/docker-image.yml/badge.svg)](https://github.com/secondary-smiles/chela/actions/workflows/docker-image.yml)
[![Docker Image Size](https://img.shields.io/docker/image-size/secondsmiles/chela)](https://hub.docker.com/r/secondsmiles/chela)

# Chela
Chela is a minimal URL shortener built in Rust. It is named after the small claw on crustaceans.

## Usage
You can create a redirect by navigating to the `/create` page and filling out the form. By default, every path passed to Chela will be treated as a redirect except `/` and `/create`.

Chela also supports basic analytics for shortened URLs. This page is available at `/tracking`, and `/tracking/<URL ID>`.

## Install and Run
### With Docker
#### CLI
```bash
$ docker run -d \
    -p 3000:3000 \
    -e DATABASE_URL=postgres://chela:password@dbhost/postgres?sslmode=disable \
    -e CHELA_HOST=a.com \
    secondsmiles/chela
```

#### Docker Compose
```yaml
services:
    chela-postgres:
        image: postgres:15
        environment:
            - POSTGRES_USER=chela
            - POSTGRES_PASSWORD=password
        volumes:
            - chela-db:/var/lib/postgresql/data
        restart: unless-stopped
    chela:
        image: secondsmiles/chela
        ports:
            - 3000:3000
        environment:
            - DATABASE_URL=postgres://chela:password@chela-postgres/postgres?sslmode=disable
            - CHELA_HOST=a.com
            - CHELA_MAIN_PAGE_REDIRECT='https://example.com'
            - CHELA_BEHIND_PROXY=1
        depends_on:
            - chela-postgres
        restart: unless-stopped

volumes:
    chela-db:
```

#### Environment Variables

##### `DATABASE_URL`
Used to define the database connection for Chela to use.

##### `CHELA_HOST`
The hostname that Chela should refer to itself as. Defaults to `localhost`.

##### `CHELA_LISTEN_ADDRESS`
The address that Chela should listen on. Defaults to `0.0.0.0`. 

##### `CHELA_MAIN_PAGE_REDIRECT`
A page that Chela will redirect to when `/` is requested instead of replying with the default homepage.

##### `CHELA_BEHIND_PROXY`
If this variable is set, Chela will use the `X-Real-IP` header as the client IP address rather than the connection address.

##### `CHELA_UNIX_SOCKET`
If you would like Chela to listen for HTTP requests over a Unix socket, set this variable to the socket path that it should use. By default, Chela will listen via a Tcp socket.

##### `CHELA_ALPHABET`
If this variable is set, Chela will use the characters in `CHELA_ALPHABET` to create IDs for URLs. The default alphabet is `abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ`. See [here](https://sqids.org/faq#unique) for more information on Sqids alphabets.

### Manually
#### Build
```bash
$ git clone https://github.com/secondary-smiles/chela.git
$ cd chela
$ cargo build -r
```

#### Run
```bash
$ export DATABASE_URL=postgres://chela:password@dbhost/postgres?sslmode=disable
$ export CHELA_HOST=a.com
$ export CHELA_LISTEN_ADDRESS=127.0.0.1
$ ./target/release/chela
```

## Hosting
Chela uses the [axum](https://crates.io/crates/axum) to manage HTTP requests, so it is possible to expose it directly to the outer internet. However, there is no authentication for the `/create` or `/tracking` endpoints so anyone will be able to create redirects and view analytics.

If you would prefer to be the only one able to access these pages, then you can proxy Chela through Nginx with http-basic-auth. Refer to [this](https://docs.nginx.com/nginx/admin-guide/security-controls/configuring-http-basic-authentication/) documentation for more information.

```nginx
server {
    server_name example.com;
    
    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header X-Real-IP $remote_addr;

        limit_except GET HEAD {
            auth_basic 'Restricted';
            auth_basic_user_file /path/to/your/.htpasswd;
        }
    }

    location /tracking {
        proxy_pass http://localhost:3000$request_uri;
        proxy_set_header X-Real-IP $remote_addr;

        auth_basic 'Restricted';
        auth_basic_user_file /path/to/your/.htpasswd;
    }
}
```
