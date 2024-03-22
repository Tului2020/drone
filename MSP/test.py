# from shell import board
from time import sleep, time

total_time = 3
start_speed = 1000
end_speed = 1400

half_time = total_time / 2
delta_speed =  end_speed - start_speed
delta_time = half_time / delta_speed
start_timestamp = time()

for i in range(start_speed, end_speed):
    time_passed = time() - start_timestamp
    speed = i
    # board.set_motor_individual(speed, speed, speed, speed)
    if (i % 1) == 0:
        print(round(time_passed), delta_time, speed)
    sleep(delta_time)

for s in range(start_speed, end_speed + 1):
    time_passed = time() - start_timestamp
    speed = end_speed - s + start_speed
    # board.set_motor_individual(speed, speed, speed, speed)
    if (i % 1) == 0:
        print(round(time_passed), delta_time, speed)
    sleep(delta_time)

# board.set_motor_individual(1000, 1000, 1000, 1000)

# 


