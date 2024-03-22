from shell import board
from time import sleep

total_time = 3
start_speed = 1000
end_speed = 1400
delta_speed =  end_speed - start_speed
delta_time = delta_speed / total_time


# for i in range(start_speed, end_speed):
#     board.set_motor_individual(i, i, i, i)
#     print(delta_time, i)
#     sleep(delta_time)

for i in range(start_speed, end_speed):
    speed = end_speed - i
    # board.set_motor_individual(speed, speed, speed, speed)
    print(delta_time, i)
    sleep(delta_time)

# board.set_motor_individual(1000, 1000, 1000, 1000)