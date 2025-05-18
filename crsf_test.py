from crsf_controller import CRSFPort
from time import sleep

# ---------------------------------------------------------------------------
# Tiny demo script -----------------------------------------------------------
if __name__ == "__main__":
    port = CRSFPort(debug=True)          # prints each frame hex
yaw = 1000
step = 6
# port.arm()                        # arm the drone
# print("----------------------- Armed, props on. ----------------------- ")
# port.beep()

try:
    for i in range(20):
        port.send_rc(yaw=yaw, pitch=yaw, aux3=1800, aux1=1800)   # gentle hover, sweep yaw
        yaw += step
        if yaw >= 2000 or yaw <= 1000:
            step = -step
        sleep(0.02)                 # 50 Hz
    port.arm()
    while True:
        port.send_rc(yaw=yaw, pitch=yaw, aux3=1800, aux1=1800)   # gentle hover, sweep yaw
        yaw += step
        if yaw >= 2000 or yaw <= 1000:
            step = -step
        sleep(0.02)      

except KeyboardInterrupt:
    # port.send_rc(thr=1000)               # props off
    print("\nStopped — failsafe low-throttle sent.")
