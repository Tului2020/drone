from shell import board
from time import sleep

total_time = 3
start_speed = 1000
end_speed = 1400
delta_speed =  end_speed - start_speed
delt_time = total_time / delta_speed


for i in range(start_speed, end_speed):
    board.set_motor(i)
    sleep(delt_time)


board.set_motor(1000)