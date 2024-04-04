# Stablize the drone using the MSP library
from shell import board
from time import sleep

MAX_SPEED = 1200
MIN_SPEED = 1000

try:
    while True:
        attitude = board.getData(board.ATTITUDE)
        angx = attitude['angx']
        angy = attitude['angy']
        heading = attitude['heading']

        # Stablize the drone
        # TEST2


        sleep(0.02)

except KeyboardInterrupt:
    print("Program stopped by user")
    board.ser.close()
    print("Serial port closed")
    exit()
