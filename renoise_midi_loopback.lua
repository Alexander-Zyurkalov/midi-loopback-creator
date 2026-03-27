local midi_loopback = require("midi_loopback")

local loopbacks      = {}  -- [instrument_index] -> MIDILoopback
local prev_names     = {}  -- [instrument_index] -> string
local name_notifiers = {}  -- [instrument_index] -> closure

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
    local notifier = function() on_name_changed(i) end
    name_notifiers[i] = notifier
    instrument.name_observable:add_notifier(notifier)
end

local function detach_all_observers()
    local song = renoise.song()
    for i, notifier in pairs(name_notifiers) do
        if i <= #song.instruments then
            local instrument = song.instruments[i]
            if instrument.name_observable:has_notifier(notifier) then
                instrument.name_observable:remove_notifier(notifier)
            end
        end
    end
    name_notifiers = {}
    prev_names     = {}
    loopbacks      = {}
end

local function setup_observers()
    detach_all_observers()
    local song = renoise.song()
    for i = 1, #song.instruments do
        attach_observer(i)
    end
end

renoise.song().instruments_observable:add_notifier(setup_observers)
renoise.tool().app_new_document_observable:add_notifier(setup_observers)