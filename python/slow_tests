#!/bin/env python
import sys
import json

# The treshold for slow tests
threshold = float(sys.argv[1])

# The test output file
path = sys.argv[2]

file1 = open(path, "r")
lines = file1.readlines()

slow_tests = []

for line in lines:
    test = json.loads(line)

    # Ignore suite entries, we're intrested in individual tests
    if test["type"] != "test":
        continue

    # Ignore any tests without an execution time
    if "exec_time" not in test:
        continue

    if test["exec_time"] > threshold:
        slow_tests.append(test)

# Sort the slow tests by execution times
slow_tests = sorted(slow_tests, key=lambda test: test["exec_time"])

print("Slow tests: (Exec_time, Name)")
for test in slow_tests:
    print(f"{test['exec_time'], test['name']}")
