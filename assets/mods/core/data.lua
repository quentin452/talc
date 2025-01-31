data = {}

function extend(prototype)
    data[prototype.type] = data[prototype.type] or {}
    data[prototype.type][prototype.name] = prototype
end
