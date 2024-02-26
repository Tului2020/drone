#!/usr/bin/env python

from msp import MultiWii
from util import push16
import time

board = MultiWii("/dev/ttyS0")
print("Flight Controller connected!")

board.enable_arm()
board.arm()


while True:
    # calculate roll, pitch, yaw and throttle from dpad positions
    x, y = 0.0, -1.0

    throttle = (y + 1.0) / 2.0
    rudder = x

    x, y = 0.0, 0.0

    aileron = x
    elevator = y

    # roll, pitch, yaw, throttle, aux1, aux2, aux3, aux4
    # each from 1000 to 2000
    buf = []
    push16(buf, int(aileron * 500 + 1500))
    push16(buf, int(elevator * 500 + 1500))
    push16(buf, int(throttle * 1000 + 1000))
    push16(buf, int(rudder * 500 + 1500))
    push16(buf, 1500)
    push16(buf, 1000)
    push16(buf, 1000)
    push16(buf, 1000)

    # send rc command
    board.sendCMD(MultiWii.SET_RAW_RC, buf)
    time.sleep(0.025)

    # print board attitude
    board.getData(MultiWii.ATTITUDE)
    print(board.attitude)
    time.sleep(0.025)