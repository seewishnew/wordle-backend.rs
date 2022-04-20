TAG ?= latest
REGISTRY ?= localhost:5000
BASE_IMAGE ?= rust:1.60

deploy-mongo:
	docker run --name some-mongo -v $(realpath data):/data/db -d -p 27017:27017 mongo:5.0.0

serve:
	$(shell SECRET_KEY=$(shell openssl rand -base64 32) cargo run --release > output.log)

docker-image:
	docker build -t wordle-backend:$(TAG) --build-arg=IMAGE=$(BASE_IMAGE) .                         
	docker tag wordle-backend:$(TAG) $(REGISTRY)/wordle-backend:$(TAG)
	docker push $(REGISTRY)/wordle-backend:$(TAG)