-- A declarative hyprland.lua fixture (the subset hyprconf manages).
local mainMod = "SUPER"

require("extra")

hl.config({
    general = {
        gaps_in = 6,
        ["col.active_border"] = "rgba(11223344)",
        layout = "master",
    },
    decoration = {
        rounding = 8,
        blur = {
            enabled = false,
        },
    },
})

-- Autostart
hl.exec_cmd("waybar")

-- Keybinds
hl.bind("SUPER, Q", "killactive")
hl.bind("SUPER, T", "exec kitty")

-- A dynamic region the GUI must preserve verbatim and never flatten.
for i = 1, 9 do
    hl.bind("SUPER, " .. i, "workspace " .. i)
end

hl.window_rule({ name = "float", match = "class:^(pavucontrol)$" })
