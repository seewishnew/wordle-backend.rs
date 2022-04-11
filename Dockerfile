ARG IMAGE=rust:1.60
FROM ${IMAGE}

WORKDIR /usr/src/wordle-backend
COPY . .

RUN cargo install --path .

CMD ["wordle-backend"]