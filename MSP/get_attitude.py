# Get the attitude of the flight controller
from shell import board
from time import sleep

try:
    while True:
        print(board.getData(board.ATTITUDE))
        sleep(0.02)

except KeyboardInterrupt:
    print("Program stopped by user")
    board.ser.close()
    print("Serial port closed")
    exit()
