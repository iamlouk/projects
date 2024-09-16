#!/usr/bin/python3
import math
import random
import bisect

N = 500
data = [int(random.random() * 100) for i in range(0, N)]

print(data)

def sorted_insert(x, arr, key=lambda x: x):
    bisect.insort(arr, x, key=key)

def sorted_replace(x, arr, key=lambda x: x):
    for i in range(0, len(arr)):
        if key(x) > key(arr[i]):
            if i < len(arr) - 1:
                next = arr[i + 1]
                if key(x) < key(next):
                    arr[i] = x
                else:
                    arr[i] = next
            else:
                arr[i] = x

def kth_element(k, data, key=lambda x: x):
    arr = []
    for x in data:
        if len(arr) < k:
            sorted_insert(x, arr, key)
        elif key(x) > key(arr[0]):
            sorted_replace(x, arr, key)

    return arr[0]

def percentile(p, data):
    data = sorted(data)
    pn = int(len(data) * p)
    return data[pn]

def percentile_fancy(p, data):
    pn = len(data) * p
    if p < 0.5:
        return kth_element(int(pn) + 1, data, key=lambda x: -x)
    else:
        return kth_element(int(len(data) - pn), data, key=lambda x: x)

print('classic:')
print(f'10%: {percentile(0.10, data)}')
print(f'25%: {percentile(0.25, data)}')
print(f'50%: {percentile(0.50, data)}')
print(f'75%: {percentile(0.75, data)}')
print(f'90%: {percentile(0.90, data)}')
print('fancy:')
print(f'10%: {percentile_fancy(0.10, data)}')
print(f'25%: {percentile_fancy(0.25, data)}')
print(f'50%: {percentile_fancy(0.50, data)}')
print(f'75%: {percentile_fancy(0.75, data)}')
print(f'90%: {percentile_fancy(0.90, data)}')
