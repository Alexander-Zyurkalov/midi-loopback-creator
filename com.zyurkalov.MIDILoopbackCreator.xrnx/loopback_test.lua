local midi_loopback = require("midi_loopback_creator")

local loopback, source_id, destination_id, err = midi_loopback.new("MyLoopback")
if not loopback then
    error("Failed to create loopback: " .. err)
end

print("Created loopback, source_id: " .. source_id .. ", destination_id: " .. destination_id)
print("Name: " .. loopback:get_name())


local _, err1 = loopback:rename("MyLoopback_Renamed")
if err1 then
    error("Failed to rename: " .. err1)
end

print("Renamed to: " .. loopback:get_name())
print("ID unchanged: " .. loopback:get_unique_id())