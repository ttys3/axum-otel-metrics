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
	sleep 1
done


