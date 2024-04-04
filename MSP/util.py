def push8(buf, val):
    buf.append(0xFF & val)


def push16(buf, val):
    # low byte
    push8(buf, val)

    # high byte
    push8(buf, val >> 8)

class PIDController:
    def __init__(self, Kp, Ki, Kd):
        self.Kp = Kp
        self.Ki = Ki
        self.Kd = Kd
        self.integral = 0
        self.previous_error = 0

    def update(self, setpoint, measured_value, time_delta):
        error = setpoint - measured_value
        self.integral += error * time_delta
        derivative = (error - self.previous_error) / time_delta
        output = self.Kp * error + self.Ki * self.integral + self.Kd * derivative
        self.previous_error = error
        return output
