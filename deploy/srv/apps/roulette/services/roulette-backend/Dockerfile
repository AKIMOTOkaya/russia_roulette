FROM debian:12-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY runtime /app/runtime

EXPOSE 8080

ENTRYPOINT ["/app/runtime/roulette-backend"]
