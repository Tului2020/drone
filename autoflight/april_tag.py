# libcamera-vid -t 0 --inline -o - | nc -l 2222
import av
import cv2
import numpy as np

STREAM_URL = "tcp://tului-drone.local:2222"

# Guidance tuning
DEADZONE_PX = 30          # how close to center counts as "centered"
GAIN = 1.0                # scales suggested movement magnitude
MAX_CMD = 100             # clamp "move" suggestion magnitude
SHOW_OPTICAL_FLOW = False # set True if you still want the flow arrows

# Optical flow tuning (only used if SHOW_OPTICAL_FLOW=True)
scale = 0.5
step = 32
prev_gray = None

def clamp(v, lo, hi):
    return max(lo, min(hi, v))

def guidance_from_center(target_xy, frame_wh):
    """
    target_xy: (cx, cy) in pixel coords
    frame_wh: (w, h)
    Returns: (dx, dy, text) where dx/dy are signed pixel offsets from center.
    """
    cx, cy = target_xy
    w, h = frame_wh
    fx, fy = w / 2.0, h / 2.0
    dx = cx - fx   # +dx means target is to the right of center
    dy = cy - fy   # +dy means target is below center

    # Within deadzone => centered
    if abs(dx) <= DEADZONE_PX and abs(dy) <= DEADZONE_PX:
        return dx, dy, "CENTERED"

    # Suggest move direction to bring target to center:
    # If target is right (+dx), you need move LEFT; if target is below (+dy), move UP.
    horiz = "LEFT" if dx > DEADZONE_PX else ("RIGHT" if dx < -DEADZONE_PX else "")
    vert  = "UP"   if dy > DEADZONE_PX else ("DOWN"  if dy < -DEADZONE_PX else "")

    # Suggested magnitude (simple proportional)
    mag_x = int(clamp(abs(dx) * GAIN, 0, MAX_CMD))
    mag_y = int(clamp(abs(dy) * GAIN, 0, MAX_CMD))

    parts = []
    if horiz:
        parts.append(f"{horiz} {mag_x}")
    if vert:
        parts.append(f"{vert} {mag_y}")

    return dx, dy, " / ".join(parts) if parts else "ADJUST"

def draw_overlay(img, target_xy, label, guidance_text):
    h, w = img.shape[:2]
    cx, cy = int(target_xy[0]), int(target_xy[1])

    # Crosshair at frame center
    cv2.drawMarker(img, (w // 2, h // 2), (255, 255, 255), markerType=cv2.MARKER_CROSS, markerSize=20, thickness=2)

    # Target marker
    cv2.circle(img, (cx, cy), 6, (0, 255, 255), -1)
    cv2.putText(img, label, (cx + 8, cy - 8), cv2.FONT_HERSHEY_SIMPLEX, 0.6, (0, 255, 255), 2, cv2.LINE_AA)

    # Arrow from target to center (shows where to move target toward)
    cv2.arrowedLine(img, (cx, cy), (w // 2, h // 2), (0, 255, 255), 2, tipLength=0.2)

    # Guidance text
    cv2.putText(img, f"GUIDE: {guidance_text}", (10, 30), cv2.FONT_HERSHEY_SIMPLEX, 0.8, (50, 255, 50), 2, cv2.LINE_AA)

def try_detect_qr(gray):
    """
    Returns: (found, center_xy, extra_draw_data)
    extra_draw_data: list of polygons (each Nx2) to draw if desired
    """
    qr = cv2.QRCodeDetector()
    # Prefer multi when available
    if hasattr(qr, "detectAndDecodeMulti"):
        ok, decoded_info, points, _ = qr.detectAndDecodeMulti(gray)
        if ok and points is not None and len(points) > 0:
            # pick the first detected QR
            pts = points[0].astype(np.float32)  # 4x2
            center = pts.mean(axis=0)
            return True, (float(center[0]), float(center[1])), [pts]
        return False, None, []
    else:
        data, pts, _ = qr.detectAndDecode(gray)
        if pts is not None and len(pts) > 0:
            pts = pts.astype(np.float32)  # 4x2
            center = pts.mean(axis=0)
            return True, (float(center[0]), float(center[1])), [pts]
        return False, None, []

def try_detect_apriltag(gray):
    """
    Uses OpenCV aruco (opencv-contrib-python).
    Returns: (found, center_xy, corners_to_draw, id_str)
    """
    if not hasattr(cv2, "aruco"):
        return False, None, None, None

    # Pick a common AprilTag dictionary (change if you use another family)
    # Options include: DICT_APRILTAG_36h11, DICT_APRILTAG_25h9, DICT_APRILTAG_16h5, etc.
    dict_name = "DICT_APRILTAG_36h11"
    if not hasattr(cv2.aruco, dict_name):
        return False, None, None, None

    dictionary = cv2.aruco.getPredefinedDictionary(getattr(cv2.aruco, dict_name))

    # Newer OpenCV has ArucoDetector
    if hasattr(cv2.aruco, "ArucoDetector"):
        params = cv2.aruco.DetectorParameters()
        detector = cv2.aruco.ArucoDetector(dictionary, params)
        corners, ids, _ = detector.detectMarkers(gray)
    else:
        params = cv2.aruco.DetectorParameters_create()
        corners, ids, _ = cv2.aruco.detectMarkers(gray, dictionary, parameters=params)

    if ids is None or len(ids) == 0:
        return False, None, None, None

    # pick first detected tag
    c = corners[0].reshape(-1, 2).astype(np.float32)  # 4x2
    center = c.mean(axis=0)
    tag_id = int(ids[0][0])
    return True, (float(center[0]), float(center[1])), c, f"APRILTAG id={tag_id}"

# Open the live stream (low-latency hint)
container = av.open(STREAM_URL, options={"flags": "low_delay"})

for packet in container.demux(video=0):
    for frame in packet.decode():
        img = frame.to_ndarray(format="bgr24")

        # Your original flip (kept)
        img = cv2.flip(cv2.flip(img, 0), 1)

        gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)

        found = False
        label = ""
        target_xy = None

        # 1) Try QR
        ok_qr, center_qr, polys = try_detect_qr(gray)
        if ok_qr:
            found = True
            target_xy = center_qr
            label = "QR"
            # Draw QR polygon
            for pts in polys:
                pts_i = pts.astype(np.int32)
                cv2.polylines(img, [pts_i], True, (0, 255, 255), 2)

        # 2) If no QR, try AprilTag
        if not found:
            ok_tag, center_tag, corners, id_str = try_detect_apriltag(gray)
            if ok_tag:
                found = True
                target_xy = center_tag
                label = id_str
                # Draw tag box
                cv2.polylines(img, [corners.astype(np.int32)], True, (255, 255, 0), 2)

        # Guidance + overlay
        if found and target_xy is not None:
            dx, dy, guide = guidance_from_center(target_xy, (img.shape[1], img.shape[0]))
            draw_overlay(img, target_xy, label, guide)

            # Also print concise guidance to console (optional)
            # print(f"{label}: dx={dx:.1f}px dy={dy:.1f}px -> {guide}")

        else:
            cv2.putText(img, "No QR/AprilTag detected", (10, 30), cv2.FONT_HERSHEY_SIMPLEX, 0.8,
                        (0, 0, 255), 2, cv2.LINE_AA)

        # Optional: keep your optical flow overlay
        if SHOW_OPTICAL_FLOW:
            small = cv2.resize(gray, (0, 0), fx=scale, fy=scale)
            if prev_gray is not None:
                prev_small = cv2.resize(prev_gray, (0, 0), fx=scale, fy=scale)

                flow = cv2.calcOpticalFlowFarneback(
                    prev_small, small, None,
                    pyr_scale=0.5, levels=2, winsize=9,
                    iterations=2, poly_n=5, poly_sigma=1.2, flags=0
                )

                hS, wS = small.shape
                y, x = np.mgrid[step//2:hS:step, step//2:wS:step].astype(np.int32)
                fx, fy = flow[y, x].T

                for (x1, y1, ddx, ddy) in zip(x.flatten(), y.flatten(), fx.flatten(), fy.flatten()):
                    x1, y1 = int(x1 / scale), int(y1 / scale)
                    ddx, ddy = int(ddx / scale), int(ddy / scale)
                    cv2.arrowedLine(img, (x1, y1), (x1 + ddx, y1 + ddy), (0, 255, 0), 1, tipLength=0.3)

            prev_gray = gray

        cv2.imshow("Live QR/AprilTag Guidance", img)
        if cv2.waitKey(1) & 0xFF == ord('q'):
            cv2.destroyAllWindows()
            raise SystemExit