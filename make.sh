#!/bin/bash

info() { echo $@>&2; }
fatal() { echo $@>&2; exit 1; }


defaults="clean build"

run() {
	if [[ "$task" = "clean" ]]; then
		info "Cleaning target dir..."
		rm -fr target
		return 0
	fi

	if [[ "$task" = "build" ]]; then
		info "Building pnpm..."
		pnpm run build || fatal "'pnpm run build' failed"

		info
		info "Generating target plugin dir..."
		target="target/decky-macro"
		mkdir -p $target
		cp -r bin dist assets main.py plugin.json package.json $target/
		return 0
	fi

	fatal "Invalid task: $task"
}

for task in $defaults; do
	run $task
	[[ $? -ne 0 ]] && fatal "Task failed: $task"
done
exit 0
	
