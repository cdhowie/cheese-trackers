# Cheese Trackers Install Guide

Anyone is welcome to run their own instance of Cheese Trackers!  The preferred
method of deployment is Docker.  Images are published to the
[`cdhowie/cheese-trackers` repository on Docker
Hub](https://hub.docker.com/r/cdhowie/cheese-trackers).

Note that everyone is welcome to use [the canonical
instance](https://cheesetrackers.theincrediblewheelofchee.se)!  If you are
considering hosting your own instance only because you use an alternative
Archipelago server that does not seem to work with it, this is because tracker
URL prefixes need to be explicitly allowed for security reasons.  Consider
contacting @theincrediblewheelofcheese on Discord to have your Archipelago
instance's tracker prefix added to the list of allowed trackers.

## Quick Start

You can use the following `docker-compose.yml` template to deploy a database server and the Cheese Trackers web service:

```yaml
services:
  tracker:
    image: docker.io/cdhowie/cheese-trackers:latest
    restart: always
    ports:
      - '127.0.0.1:8080:8080'
    volumes:
      - './config.yaml:/app/config.yaml:ro'

  database:
    image: postgres:17.4
    restart: always
    environment:
      POSTGRES_USER: cheesetrackers
      POSTGRES_PASSWORD: cheesetrackers
      POSTGRES_DB: cheesetrackers
    volumes:
      - './pgdata:/var/lib/postgresql/data'
```

The format of the `config.yaml` file is covered in the next section.

If you want to always follow the most recent release, you can use the `latest`
tag as in the sample file above.  However, you can also select a timestamp-based
tag or a commit-id-based tag instead if you want to stick to a specific version.

Note that it is expected that a reverse proxy (such as nginx) will be used to
perform TLS termination, so the above configuration binds the web service's
listening port to the loopback address to keep it from being reachable remotely.

## Configuration File

Here is a template you can use to populate the `config.yaml` file:

```yaml
# Address to listen on.  The port number should match the last section of the
# "ports" item from docker-compose.yml.
http_listen: '0.0.0.0:8080'

# Public URL of the instance.  This is required by Discord OAuth so that it
# knows where to redirect users after authentication is complete.
public_url: 'https://example.com/'

# Name of the entity providing the service.  This will be displayed in the
# footer so that users know who to contact for support.
hoster: An Unknown Gaming Community

# Database configuration.
database:
  # Database type.  Right now "postgres" is the only option, but other
  # databases may be supported later.
  type: postgres
  # Connection string.  The exact format will depend on the database type.
  #
  # For PostgreSQL, see the relevant sqlx documentation:
  #
  # https://docs.rs/sqlx/latest/sqlx/postgres/struct.PgConnectOptions.html
  #
  # The string below will work with the quick start docker-compose.yml.
  connection_string: 'postgresql://cheesetrackers:cheesetrackers@database/cheesetrackers'

# The source for client IP address information, used to populate the audit
# log.  Depending on your reverse proxy and/or CDN configuration, this may
# need to be adjusted.
#
# Possible values:
#
# - CfConnectingIp: Uses the CF-Connecting-IP HTTP header, useful when the
#   service is deployed behind Cloudflare.
# - CloudFrontViewerAddress: Uses the CloudFront-Viewer-Address HTTP header,
#   useful when the service is deployed behind AWS CloudFront.
# - ConnectInfo: Uses the real remote address of the connecting socket.  This
#   will be incorrect when a reverse proxy is involved.
# - FlyClientIp: Uses the Fly-Client-IP HTTP header, useful when the service
#   is deployed behind Fly.
# - RightmostForwarded: Uses the rightmost address from the standard Forwarded
#   HTTP header.
# - RightmostXForwardedFor: Uses the rightmost address from the
#   pseudo-standard X-Forwarded-For HTTP header.
# - TrueClientIp: Uses the rightmost address from the standard True-ClientIP
#   HTTP header.
# - XRealIp: Uses the pseudo-standard X-Real-IP HTTP header.
client_ip_source: ConnectInfo

# Enables CORS permissive headers.  This is mostly useful during development,
# otherwise the origin of the web service and frontend should be the same.
cors_permissive: false

# Sets the minimum interval in minutes between tracker updates.  If a tracker
# is refreshed within this interval from the last update, the API will skip
# fetching the AP web tracker data and return cached data from the database.
tracker_update_interval_mins: 1

# Allowed upstream trackers.  Requests for trackers that do not begin with an
# item of this list will be denied.  This prevents a confused deputy
# vulnerability where users could misuse the web service to send GET requests
# to arbitrary URLs.
upstream_trackers:
  - 'https://archipelago.gg/tracker'

# List of banners to show at the top of the UI.  This is a list of objects.
#
# Each banner object has the following keys:
#
# - id: Text, optional.  If present, the user will be allowed to dismiss the
#   message.  The IDs of dismissed banners are stored in the user's browser's
#   local storage, so ensure that the IDs of every banner that you've ever
#   used is unique.  Randomly-generated UUIDs would be a good choice.
# - message: Text, required.  The content of the banner, which can contain
#   HTML.
# - kind: Text, required.  Specifies the kind of banner, which may be one of:
#   danger, warning, success, or info.
banners: []

# Authentication token configuration.
token:
  # Secret used to sign tokens.  Generate a long, random string and do not
  # share it.  Changing the token secret will invalidate everyone's
  # authentication tokens and effectively sign them out.
  secret: changeme
  # Token issuer.  You can technically use any value here, but best practice
  # would be the public domain name of the instance.
  issuer: 'example.com'
  # Token validity in days.  Tokens will be valid this long and then expire,
  # which will require the user to authenticate again.
  validity_duration_days: 90
  # Token signature algorithm.  The default is HS265.
  #algorithm: HS265

# Discord OAuth.
discord:
  # Discord app client ID.
  client_id: 0
  # Discord app secret.
  client_secret: ''
  # Cipher key used to encrypt the continuation token.  This must be a
  # base64-encoded sequence of 32 bytes randomly generated from a
  # cryptographically secure PRNG.
  token_cipher_key: ''
```

## Reverse Proxy

Cheese Trackers is designed to run behind a reverse proxy.  In particular, it
does not provide TLS support, which should be handled by the reverse proxy.

Here is a sample nginx configuration.  If using this configuration (specifically
the `proxy_set_header X-Real-IP $remote_addr` line) then you should specify
`client_ip_source: XRealIp` in the web service configuration file.

```nginx
server {
    listen 80;
    listen [::]:80;

    server_name example.com;

    location / {
        return 301 https://$host$request_uri;
    }
}

server {
    listen 443 ssl http2;
    listen [::] 443 ssl http2;

    server_name example.com;

    ssl_certificate /path/to/certificate.crt;
    ssl_certificate_key /path/to/certificate.key;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```
