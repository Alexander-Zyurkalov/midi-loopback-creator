local midi_loopback = require("midi_loopback_creator")

local loopbacks  = {}  -- [instrument_index] -> MIDILoopback
local prev_names = {}  -- [instrument_index] -> string

local function matches_pattern(name)
    return name:match("%-midiext$") ~= nil
end

local function on_name_changed(i)
    local song = renoise.song()
    if i > #song.instruments then return end

    local new_name = song.instruments[i].name
    local old_name = prev_names[i] or ""

    if matches_pattern(new_name) then
        if matches_pattern(old_name) and loopbacks[i] then
            local _, err = loopbacks[i]:rename(new_name)
            if err then
                renoise.app():show_error(("MIDILoopback rename failed: %s"):format(err))
            end
        else
            local obj, unique_id, err = midi_loopback.new(new_name, i)
            if obj then
                loopbacks[i] = obj
                print(("MIDILoopback created for instrument %d '%s' (unique_id=%d)"):format(
                    i, new_name, unique_id))
            else
                renoise.app():show_error(
                    ("Failed to create MIDILoopback for '%s': %s"):format(new_name, err))
            end
        end
    end

    prev_names[i] = new_name
end

local function attach_observer(i)
    local instrument = renoise.song().instruments[i]
    prev_names[i] = instrument.name
    instrument.name_observable:add_notifier(function() on_name_changed(i) end)
end

local function on_instruments_changed(change)
    if change.type == "insert" then
        attach_observer(change.index)
    elseif change.type == "remove" then
        prev_names[change.index] = nil
        loopbacks[change.index]  = nil
    elseif change.type == "swap" then
        prev_names[change.index1], prev_names[change.index2] =
            prev_names[change.index2], prev_names[change.index1]
        loopbacks[change.index1], loopbacks[change.index2] =
            loopbacks[change.index2], loopbacks[change.index1]
    end
end

local function setup_observers()
    prev_names = {}
    loopbacks  = {}
    local song = renoise.song()
    for i = 1, #song.instruments do
        attach_observer(i)
    end
    song.instruments_observable:add_notifier(on_instruments_changed)
end

renoise.tool().app_new_document_observable:add_notifier(setup_observers)