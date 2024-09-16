#!/usr/bin/env python

# A, X -> Rock,
# B, Y -> Paper,
# C, Z -> Scissiors
outcomes = dict([
    ('AX', (3 + 1)),
    ('AY', (6 + 2)),
    ('AZ', (0 + 3)),
    
    ('BX', (0 + 1)),
    ('BY', (3 + 2)),
    ('BZ', (6 + 3)),
    
    ('CX', (6 + 1)),
    ('CY', (0 + 2)),
    ('CZ', (3 + 3)),
])

# Part 2:
# A, X -> Rock,
# B, Y -> Paper,
# C, Z -> Scissiors
# X -> Lose,
# Y -> Draw,
# Z -> Win
secret_strategy = dict([
    ('AX', 'Z'), # lose against rock
    ('AY', 'X'), # draw against rock
    ('AZ', 'Y'), # win  against rock

    ('BX', 'X'), # lose against paper
    ('BY', 'Y'), # draw against paper
    ('BZ', 'Z'), # win  against paper
    
    ('CX', 'Y'), # lose against scissors
    ('CY', 'Z'), # draw against scissors
    ('CZ', 'X'), # win  against scissors
])


total_score1, total_score2 = 0, 0
with open('./02-input.txt', 'r') as f:
    for line in f.readlines():
        line = line.strip()
        if len(line) == 0:
            break

        elfsturn, myturn = line[0], line[2]
        assert elfsturn == 'A' or elfsturn == 'B' or elfsturn == 'C'
        assert myturn == 'X' or myturn == 'Y' or myturn == 'Z'

        total_score1 += outcomes[elfsturn+myturn]

        myturn = secret_strategy[elfsturn+myturn]
        total_score2 += outcomes[elfsturn+myturn]

print(f"total score (part 1): {total_score1}")
print(f"total score (part 2): {total_score2}")
