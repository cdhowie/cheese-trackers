# Cheese Trackers

An enhanced async room tracker for [Archipelago](https://archipelago.gg).  The
canonical instance is hosted at
[cheesetrackers.theincrediblewheelofchee.se](https://cheesetrackers.theincrediblewheelofchee.se/).

Based on the fantastic Google spreadsheet built by RadzPrower.

Provides multiple enhancements on top of Archipelago's own web tracker, such as:

* Participants can:
    * Claim their slots.
    * Indicate whether their slots are BK, go mode, etc.
    * Specify how important hinted items are.
    * Make free-form text notes about their slot.
    * Specify under what conditions they would like to be pinged on Discord.
* Provides several summary views about the overall state of the room.
* Slots can be filtered and sorted various ways.
* Supports Discord authentication.

## Architecture

The tracker is split into two primary components: the backend web service and
the frontend application that runs in the web browser.

### Backend

The backend web service is written in Rust using the Axum framework.  Currently
only PostgreSQL is supported as the data store, but the data access design is
abstracted such that it would be fairly straightforward to add support for other
data stores.

### Frontend

The frontend web application is written in Vue and uses Bootstrap.

## Development

If you are interested in working on Cheese Trackers, you will want to start by
deploying a development environment.  The `podman/create-pod` script will create
a local development environment consisting of:

* A Vite server to serve the frontend.
* A PostgreSQL instance.
* A pgadmin instance for inspecting the PostgreSQL database.

Note that there is no container for hosting the web service.  Currently it's
expected that you'd run the web service outside of a container, listening on
port 3000.  This may change in the future.

After the development pod is created, you can run `podman port -a` to get a list
of the local listening ports that are mapped into the containers.  To access the
frontend, you'd look for the line containing `81/tcp` and visit the IP address
and port shown on that same line.

## Deployment

Containers are used to deploy the application.  The `build-image` script will
use podman to build an image containing both the Rust web service and the
compiled frontend.  The web service will serve the frontend's static assets, so
there is no need for a separate container to serve the frontend.