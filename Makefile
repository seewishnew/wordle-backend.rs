deploy-mongo:
	docker run --name some-mongo -v $(realpath data):/data/db -d -p 27017:27017 mongo:5.0.0

serve: deploy-mongo
	cargo run