local midi_loopback = require("midi_loopback_creator")

local loopbacks = {}  -- [instrument_index] -> MIDILoopback
local loopback_ids = {}  -- [instrument_index] -> {source_id, destination_id}
local prev_names = {}  -- [instrument_index] -> string

local ADDITIONAL_DEVICE_NAME = "H2MIDI-Pro Port 2"
local ADDITIONAL_DEVICE_NAME_2 = "Exquis"
local additional_device_id = nil  -- resolved once at load time
local additional_device_id_2 = nil  -- resolved once at load time

local function resolve_additional_device_id()
    additional_device_id = midi_loopback.find_destination_id_by_name(ADDITIONAL_DEVICE_NAME)
    if additional_device_id then
        print(("Additional MIDI device '%s' found with ID=%d"):format(
                ADDITIONAL_DEVICE_NAME, additional_device_id))
    else
        print(("Additional MIDI device '%s' not found"):format(ADDITIONAL_DEVICE_NAME))
    end
    additional_device_id_2 = midi_loopback.find_destination_id_by_name(ADDITIONAL_DEVICE_NAME_2)
    if additional_device_id_2 then
        print(("Additional MIDI device '%s' found with ID=%d"):format(
                ADDITIONAL_DEVICE_NAME_2, additional_device_id_2))
    else
        print(("Additional MIDI device '%s' not found"):format(ADDITIONAL_DEVICE_NAME_2))
    end
end

local function matches_pattern(name)
    return name:match("%-midiext$") ~= nil
end

-- ── Comment-based ID persistence ────────────────────────────────────
-- Format: "midiext|<instrument_name>|<source_id>|<destination_id>"

local function find_saved_ids(name)
    for _, c in ipairs(renoise.song().comments) do
        local n, src, dst = c:match("^midiext|(.+)|(%d+)|(%d+)$")
        if n == name then
            return tonumber(src), tonumber(dst)
        end
    end
    return nil, nil
end

local function save_ids(name, source_id, destination_id)
    local song = renoise.song()
    local comments = song.comments
    local entry = ("midiext|%s|%d|%d"):format(name, source_id, destination_id)
    for i, c in ipairs(comments) do
        if c:match("^midiext|(.+)|%d+|%d+$") == name then
            comments[i] = entry
            song.comments = comments
            return
        end
    end
    comments[#comments + 1] = entry
    song.comments = comments
end

local function delete_saved_ids(name)
    local song = renoise.song()
    local comments = song.comments
    local new_comments = {}
    for _, c in ipairs(comments) do
        if c:match("^midiext|(.+)|%d+|%d+$") ~= name then
            new_comments[#new_comments + 1] = c
        end
    end
    song.comments = new_comments
end

-- ── MIDI output assignment ───────────────────────────────────────────

local function assign_midi_output(i, name)
    local instrument = renoise.song().instruments[i]
    local props = instrument.midi_output_properties
    props.instrument_type = renoise.InstrumentMidiOutputProperties.TYPE_EXTERNAL
    props.device_name = name
end

-- ── Instrument name change handler ──────────────────────────────────

local function on_name_changed(i)
    local song = renoise.song()
    if i > #song.instruments then
        return
    end

    local new_name = song.instruments[i].name
    local old_name = prev_names[i] or ""

    if matches_pattern(new_name) then
        if matches_pattern(old_name) and loopbacks[i] then
            local _, err = loopbacks[i]:rename(new_name)
            if err then
                local msg = ("MIDILoopback rename failed: %s"):format(err)
                renoise.app():show_error(msg)
                print(msg)
            else
                local ids = loopback_ids[i]
                delete_saved_ids(old_name)
                save_ids(new_name, ids[1], ids[2])
            end
        else
            local src_id, dst_id = find_saved_ids(new_name)
            local obj, source_id, destination_id, err = midi_loopback.new(new_name, src_id, dst_id)
            if obj then
                loopbacks[i] = obj
                loopback_ids[i] = { source_id, destination_id }
                save_ids(new_name, source_id, destination_id)
                print(("MIDILoopback created for instrument %d '%s' (source_id=%d, destination_id=%d)"):format(
                        i, new_name, source_id, destination_id))
            else
                renoise.app():show_error(
                        ("Failed to create MIDILoopback for '%s': %s"):format(new_name, err))
            end
        end

        local timer_func
        timer_func = function()
            assign_midi_output(i, new_name)
            renoise.tool():remove_timer(timer_func)
        end
        renoise.tool():add_timer(timer_func, 3000)
    elseif matches_pattern(old_name) then
        delete_saved_ids(old_name)
        loopbacks[i] = nil
        loopback_ids[i] = nil
        collectgarbage("collect")
    end

    prev_names[i] = new_name
end

-- ── Observer management ─────────────────────────────────────────────

local function attach_observer(i)
    local instrument = renoise.song().instruments[i]
    prev_names[i] = instrument.name
    instrument.name_observable:add_notifier(function()
        on_name_changed(i)
    end)
    on_name_changed(i)
end

local function on_instruments_changed(change)
    if change.type == "insert" then
        attach_observer(change.index)
    elseif change.type == "remove" then
        local name = prev_names[change.index]
        if name and matches_pattern(name) then
            delete_saved_ids(name)
        end
        prev_names[change.index] = nil
        loopbacks[change.index] = nil
        loopback_ids[change.index] = nil
        collectgarbage("collect")
    elseif change.type == "swap" then
        local i1, i2 = change.index1, change.index2
        prev_names[i1], prev_names[i2] = prev_names[i2], prev_names[i1]
        loopbacks[i1], loopbacks[i2] = loopbacks[i2], loopbacks[i1]
        loopback_ids[i1], loopback_ids[i2] = loopback_ids[i2], loopback_ids[i1]
    end
end

local function setup_observers()
    prev_names = {}
    loopbacks = {}
    loopback_ids = {}
    collectgarbage("collect")
    local song = renoise.song()
    for i = 1, #song.instruments do
        attach_observer(i)
    end
    song.instruments_observable:add_notifier(on_instruments_changed)
end

renoise.tool().app_new_document_observable:add_notifier(setup_observers)

local function instrument_index_from_cursor()
    local song = renoise.song()
    local pattern = song:pattern(song.selected_pattern_index)
    local track = pattern:track(song.selected_track_index)
    local line = track:line(song.selected_line_index)
    local col_idx = song.selected_note_column_index
    if col_idx == 0 then
        return nil
    end  -- cursor is on an effect column
    local note_col = line:note_column(col_idx)
    local iv = note_col.instrument_value
    if iv == 255 then
        return nil
    end  -- cell is empty
    return iv + 1  -- Renoise stores 0-based instrument index; song.instruments is 1-based
end

local function focus_additional_device_on_current()
    resolve_additional_device_id()
    local selected = instrument_index_from_cursor()
    if not selected then
        renoise.app():show_status("MIDI Loopback: no instrument in current cell")
        return
    end

    for i, lb in pairs(loopbacks) do
        local _, err
        if i == selected then
            _, err = lb:set_additional_destination(additional_device_id, additional_device_id_2)
        else
            _, err = lb:set_additional_destination(nil, nil)
        end
        if err then
            local msg = ("set_additional_destination failed for instrument %d: %s"):format(i, err)
            renoise.app():show_error(msg)
            print(msg)
        end
    end
end

renoise.tool():add_menu_entry {
    name = "Main Menu:Tools:MIDI Loopback Creator:Focus Additional Device on Current Instrument",
    invoke = focus_additional_device_on_current,
}

renoise.tool():add_keybinding {
    name = "Global:Tools:MIDI Loopback Creator - Focus Additional Device on Current Instrument",
    invoke = focus_additional_device_on_current,
}
