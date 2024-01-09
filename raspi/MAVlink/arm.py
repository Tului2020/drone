from pymavlink import mavutil
from connection import mav_connection

mav_connection.mav.command_long_send(
    mav_connection.target_system, 
    mav_connection.target_component, 
    mavutil.mavlink.MAV_CMD_COMPONENT_ARM_DISARM, 
    *[0, 1, 0, 0, 0, 0, 0, 0]
)

print(mav_connection.recv_match(type='COMMAND_ACK', blocking=True))