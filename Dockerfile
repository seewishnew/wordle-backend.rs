FROM rust:1.60

WORKDIR /usr/src/wordle-backend
COPY . .

RUN cargo install --path .

CMD ["wordle-backend"]