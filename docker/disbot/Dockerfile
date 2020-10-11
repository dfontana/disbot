####################
# 0. Prep Wire
####################
FROM golang:1.15.2 as wire_builder
RUN go get -u github.com/google/wire/cmd/wire

####################
# 1. Build
####################
FROM wire_builder as builder
WORKDIR /build
COPY ./src/ /build/
RUN wire ./...
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build

###################
# 2. Run
###################
FROM alpine
COPY --from=builder /build/disbot /app
ENTRYPOINT ["/app"]
