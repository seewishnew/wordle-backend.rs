URL ?= localhost:8080
N ?= 100
all: user-id create-game manage-game register play get-state frontend

user-id:
	hyperfine -w 3 -r $(N) --export-json user_id.json 'curl -c wordle.cookie -X POST -d '"'"'{"name": "seewishnew"}'"'"' $(URL)/api/v1/user_id'

verify-user-id:
	curl -c wordle.cookie -X POST -d '{"name": "seewishnew"}' $(URL)/api/v1/user_id
	hyperfine --show-output -w 3 -r $(N) --export-json verify_user_id.json \
		'curl $(URL)/api/v1/user_id/verify' 'curl -vv -b wordle.cookie $(URL)/api/v1/user_id/verify'

create-game:
	hyperfine -w 3 -r $(N) --export-json create_game.json 'curl -b wordle.cookie -X POST -d '"'"'{"answer": "hello"}'"'"' $(URL)/api/v1/create'

manage-game:
	curl -c wordle.cookie -X POST -d '{"name": "seewishnew"}' $(URL)/api/v1/user_id
	GAME_ID="$(shell curl -b wordle.cookie -X POST -d '{"answer": "hello"}' $(URL)/api/v1/create | jq -r '.game_id')"; \
					hyperfine -w 3 -r $(N) --export-json manage_game.json "curl -b wordle.cookie -X GET $(URL)/api/v1/manage/$${GAME_ID}"

register:
	curl -c wordle.cookie -X POST -d '{"name": "seewishnew"}' $(URL)/api/v1/user_id
	GAME_ID="$(shell curl -b wordle.cookie -X POST -d '{"answer": "hello"}' $(URL)/api/v1/create | jq -r '.game_id')"; \
					hyperfine -w 3 -r $(N) --export-json register.json \
					--prepare 'curl -c wordle.cookie -X POST -d '"'"'{"name": "wenhsiwees"}'"'"' $(URL)/api/v1/user_id'\
					"curl -b wordle.cookie -X POST $(URL)/api/v1/game/$${GAME_ID}/register"

play:
	curl -c wordle.cookie -X POST -d '{"name": "seewishnew"}' $(URL)/api/v1/user_id
	GAME_ID="$(shell curl -b wordle.cookie -X POST -d '{"answer": "hello"}' $(URL)/api/v1/create | jq -r '.game_id')"; \
					curl -c wordle.cookie -X POST -d '{"name": "wenhsiwees"}' $(URL)/api/v1/user_id; \
					curl -b wordle.cookie -X POST "$(URL)/api/v1/game/$${GAME_ID}/register"; \
					hyperfine -w 3 -r $(N) --export-json play.json "curl -b wordle.cookie -X POST -d '"'{"guess": ["h", "o", "t", "e", "l"]}'"' $(URL)/api/v1/game/$${GAME_ID}/play"


get-state:
	curl -c wordle.cookie -X POST -d '{"name": "seewishnew"}' $(URL)/api/v1/user_id
	GAME_ID="$(shell curl -b wordle.cookie -X POST -d '{"answer": "hello"}' $(URL)/api/v1/create | jq -r '.game_id')"; \
					curl -c wordle.cookie -X POST -d '{"name": "wenhsiwees"}' $(URL)/api/v1/user_id; \
					curl -b wordle.cookie -X POST "$(URL)/api/v1/game/$${GAME_ID}/register"; \
					for i in 1 2 3; do \
						curl -b wordle.cookie -X POST -d '{"guess": ["h", "o", "t", "e", "l"]}' "$(URL)/api/v1/game/$${GAME_ID}/play"; \
					done; \
					hyperfine -w 3 -r $(N) --export-json get_state.json "curl -b wordle.cookie -X GET $(URL)/api/v1/game/$${GAME_ID}/state"

frontend:
	hyperfine -w 3 -r $(N) --export-json frontend.json 'curl -X GET $(URL)' 'curl -b wordle.cookie -X GET $(URL)'

average: all
	jq -s '[.[].results[].mean] | add / length' {user_id,verify_user_id,create_game,manage_game,register,play,get_state,frontend}.json
