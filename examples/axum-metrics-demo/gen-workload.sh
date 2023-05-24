#!/usr/bin/env bash

set -eou pipefail

#!/bin/bash
while :
do
	echo "Press [CTRL+C] to stop.."
    curl -Ssf http://127.0.0.1:3000/

    curl -Ssf http://127.0.0.1:3000/hello
    curl -Ssf http://127.0.0.1:3000/hello

    curl -Ssf http://127.0.0.1:3000/world
    curl -Ssf http://127.0.0.1:3000/world
    curl -Ssf http://127.0.0.1:3000/world

    data=$(printf %3000s |tr " " "x"  )
    curl -Ssf http://127.0.0.1:3000/post -d $data
	sleep 1
done


