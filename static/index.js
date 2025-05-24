const defaultValues = {
    roll: 1500,
    pitch: 1500,
    yaw: 1500,
    thr: 885,
    aux1: 1000,
    aux2: 1000,
    aux3: 1000,
    aux4: 1000,
};

const keyboardMaps = {
    ArrowUp: { name: "pitch", type: 'incremental', increaseBy: 10 },
    ArrowDown: { name: "pitch", type: 'incremental', increaseBy: -10 },
    ArrowLeft: { name: "roll", type: 'incremental', increaseBy: -10 },
    ArrowRight: { name: "roll", type: 'incremental', increaseBy: 10 },
    w: { name: "thr", type: 'incremental', increaseBy: 10 },
    s: { name: "thr", type: 'incremental', increaseBy: -10 },
    a: { name: "yaw", type: 'incremental', increaseBy: -10 },
    d: { name: "yaw", type: 'incremental', increaseBy: 10 },
    1: { name: "aux1", type: 'switch', off: 1000, on: 1700 },
    2: { name: "aux2", type: 'switch', off: 1000, on: 1700 },
    3: { name: "aux3", type: 'switch', off: 1000, on: 1700 },
    4: { name: "aux4", type: 'switch', off: 1000, on: 1700 },
}

const keyboardControls = () => {
    window.addEventListener("keydown", (event) => {
        const keyboardMap = keyboardMaps[event.key];

        if (keyboardMap.type === 'incremental') {
            const { name, increaseBy } = keyboardMap;
            const input = document.getElementById(name);
            if (input) {
                let newValue = parseInt(input.value) + increaseBy;
                // Ensure the value stays within the range
                newValue = Math.max(885, Math.min(2000, newValue));
                input.value = newValue;
                input.dispatchEvent(new Event('input')); // Trigger the input event
            }

        } else if (keyboardMap.type === 'switch') {
            const { name, off, on } = keyboardMap;
            const input = document.getElementById(name);

            if (input) {
                // Toggle between off and on values
                input.value = (input.value == off) ? on : off;
                input.dispatchEvent(new Event('input')); // Trigger the input event
            }

        } else {
            console.warn("Unknown keyboard control type:", keyboardMap.type);
        }
    })
}

const main = () => {
    const slidersDiv = document.getElementById("sliders");
    const data = {};

    function sendData() {
        fetch("/set-rc", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(data),
        });
    }

    Object.entries(defaultValues).forEach(([name, defaultValue]) => {
        const container = document.createElement("div");
        container.className = "slider-container";

        const label = document.createElement("label");
        label.setAttribute("for", name);
        label.textContent = name.charAt(0).toUpperCase() + name.slice(1);

        const input = document.createElement("input");
        input.type = "range";
        input.id = name;
        input.min = 885;
        input.max = 2000;
        input.value = defaultValue;

        const valueDisplay = document.createElement("div");
        valueDisplay.className = "value";
        valueDisplay.textContent = defaultValue;

        input.addEventListener("input", () => {
            valueDisplay.textContent = input.value;
            data[name] = parseInt(input.value);
            sendData();
        });

        container.appendChild(label);
        container.appendChild(input);
        container.appendChild(valueDisplay);
        slidersDiv.appendChild(container);

        data[name] = defaultValue;
    });

    keyboardControls();
}


// Send once on load
window.addEventListener("load", main);