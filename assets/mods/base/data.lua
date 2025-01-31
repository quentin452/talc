extend {
    type = "block",
    name = "air",
    order = "a[blocks]-a[air]",
    is_transparent = true,
    is_meshable = false,
    color = {1, 1, 1}
}

extend {
    type = "block",
    name = "grass",
    order = "a[blocks]-b[grass]",
    is_transparent = false,
    is_meshable = true,
    color = {1, 1, 1}
}

extend {
    type = "block",
    name = "dirt",
    order = "a[blocks]-c[dirt]",
    is_transparent = false,
    is_meshable = true,
    color = {1, 1, 1}
}
