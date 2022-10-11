#!/usr/bin/env bash

ls -1 src/**.py  | xargs -I % reorder-python-imports %
black src