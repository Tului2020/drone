from board import board
from util import push16
import time

time.sleep(1.0)
board.enable_arm()
board.arm()

while True:
    buf = []
    push16(buf, 1500)
    push16(buf, 1500)
    push16(buf, 1500)
    push16(buf, 1500)
    push16(buf, 1500)
    push16(buf, 1000)
    push16(buf, 1000)
    push16(buf, 1000)
    board.sendCMD(board.SET_RAW_RC, buf)
    time.sleep(0.05)