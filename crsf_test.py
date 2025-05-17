from crsf_controller import CRSFPort

# ---------------------------------------------------------------------------
# Tiny demo script -----------------------------------------------------------
if __name__ == "__main__":
    port = CRSFPort(debug=True)          # prints each frame hex
    yaw = 1000
    step = 6

    try:
        while True:
            port.arm()                        # arm the drone
            # port.send_rc(yaw=yaw, thr=1280)   # gentle hover, sweep yaw
            # yaw += step
            # if yaw >= 2000 or yaw <= 1000:
            #     step = -step
            # time.sleep(0.02)                 # 50 Hz
    except KeyboardInterrupt:
        port.send_rc(thr=1000)               # props off
        print("\nStopped — failsafe low-throttle sent.")
