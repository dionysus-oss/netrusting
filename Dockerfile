FROM rust:1.64-alpine

RUN apk add --update musl-dev net-tools busybox-extras

WORKDIR /netrusting
