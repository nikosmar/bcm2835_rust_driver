from math import sqrt


numbers_to_read = 1_000
threshold = 1.96
times_exceeded = 0
mean = 0xFFFF_FFFF / 2

repetitions = 500
for _ in range(repetitions):
    n1 = 0
    n2 = 0
    # just for the initialization assume that every number
    # belongs in bin 1, change to bin 2 when necessary
    binned_numbers = [1 for _ in range(numbers_to_read)]

    # get numbers_to_read 32bit numbers from /dev/hwrng
    with open('/dev/hwrng','rb') as rng:
        for i in range(numbers_to_read):
            number = int.from_bytes(rng.read(4), "little")
            
            if number > mean:
                n2 += 1
                # change bin
                binned_numbers[i] = 2
            else:
                n1 += 1
        
    # calculate the expected number of runs
    expected_runs = 2 * n1 * n2 / numbers_to_read + 1

    # calculate the standard deviation of the number of runs.
    std_dev = sqrt((expected_runs - 1) * (expected_runs - 2) / 
                   (numbers_to_read - 1))

    # count number of runs
    observed_runs = 0
    previous_number = 0
    for number in binned_numbers:
        if number != previous_number:
            previous_number = number
            observed_runs += 1

    weight = 0.5 if observed_runs < expected_runs else -0.5
    z = (observed_runs - expected_runs + weight) / std_dev
    
    if abs(z) > threshold:
        times_exceeded += 1

print(f"{times_exceeded * 100 / repetitions}%")

