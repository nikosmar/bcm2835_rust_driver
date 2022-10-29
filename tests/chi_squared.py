numbers_to_read = 10_000
k = 10
threshold = 16.919
times_exceeded = 0
exp_i = numbers_to_read / k

repetitions = 100
for _ in range(repetitions):
    classes = [0 for _ in range(k)]
    
    # get numbers_to_read 32bit numbers from /dev/hwrng
    with open('/dev/hwrng','rb') as rng:
        for _ in range(numbers_to_read):
            number = int.from_bytes(rng.read(4), "little")
            classes[number % k] += 1

    chi_sq = 0
    for obs_i in classes:
        chi_sq += (obs_i - exp_i) ** 2 / exp_i

    if chi_sq > threshold:
        times_exceeded += 1

print(f"{times_exceeded * 100 / repetitions}%")
