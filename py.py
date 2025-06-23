# libcamera-vid -t 0 --inline -o - | nc -l 2222
import av
import cv2
import numpy as np

# Open the live stream
container = av.open("tcp://drone.local:2222", options={"flags": "low_delay"})

prev_gray = None
scale = 0.5
step = 32

for packet in container.demux(video=0):
    for frame in packet.decode():
        img = frame.to_ndarray(format="bgr24")
        img = cv2.flip(cv2.flip(img, 0), 1)
        gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)

        small = cv2.resize(gray, (0, 0), fx=scale, fy=scale)
        if prev_gray is not None:
            prev_small = cv2.resize(prev_gray, (0, 0), fx=scale, fy=scale)

            flow = cv2.calcOpticalFlowFarneback(prev_small, small, None,
                                                pyr_scale=0.5, levels=2, winsize=9,
                                                iterations=2, poly_n=5, poly_sigma=1.2, flags=0)

            h, w = small.shape
            y, x = np.mgrid[step//2:h:step, step//2:w:step].astype(np.int32)
            fx, fy = flow[y, x].T

            for (x1, y1, dx, dy) in zip(x.flatten(), y.flatten(), fx.flatten(), fy.flatten()):
                x1, y1 = int(x1 / scale), int(y1 / scale)
                dx, dy = int(dx / scale), int(dy / scale)
                cv2.arrowedLine(img, (x1, y1), (x1 + dx, y1 + dy), (0, 255, 0), 1, tipLength=0.3)

        prev_gray = gray

        cv2.imshow("Live Optical Flow (Upside Down)", img)
        if cv2.waitKey(1) & 0xFF == ord('q'):
            break

cv2.destroyAllWindows()