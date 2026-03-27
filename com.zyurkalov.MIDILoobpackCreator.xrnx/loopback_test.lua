local midi_loopback = require("midi_loopback")

local loopback, unique_id, err = midi_loopback.new("MyLoopback", 1)
if not loopback then
    error("Failed to create loopback: " .. err)
end

print("Created loopback, unique_id: " .. unique_id)
print("Name: " .. loopback:get_name())


local _, err = loopback:rename("MyLoopback_Renamed")
if err then
    error("Failed to rename: " .. err)
end

print("Renamed to: " .. loopback:get_name())
print("ID unchanged: " .. loopback:get_unique_id())