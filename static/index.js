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

const sliderAmount = 100;

const keyboardMaps = {
    arrowup: { name: "pitch", type: 'incremental', increaseBy: sliderAmount },
    arrowdown: { name: "pitch", type: 'incremental', increaseBy: -sliderAmount },
    arrowleft: { name: "roll", type: 'incremental', increaseBy: -sliderAmount },
    arrowright: { name: "roll", type: 'incremental', increaseBy: sliderAmount },
    w: { name: "thr", type: 'incremental', increaseBy: sliderAmount },
    s: { name: "thr", type: 'incremental', increaseBy: -sliderAmount },
    a: { name: "yaw", type: 'incremental', increaseBy: -sliderAmount },
    d: { name: "yaw", type: 'incremental', increaseBy: sliderAmount },
    1: { name: "aux1", type: 'modes', modes: [1000, 1700, 1950] },
    2: { name: "aux2", type: 'modes', modes: [1000, 1400, 1900] },
    3: { name: "aux3", type: 'modes', modes: [1000, 1700] },
    4: { name: "aux4", type: 'modes', modes: [1000, 1700] },
}

const keyboardControls = () => {
    window.addEventListener("keydown", (event) => {
        const keyboardMap = keyboardMaps[event.key.toLowerCase()];
        if (!keyboardMap) {
            return;
        }

        if (keyboardMap.type === 'incremental') {
            const { name, increaseBy } = keyboardMap;
            const input = document.getElementById(name);
            if (input) {
                let newValue = parseInt(input.value) + increaseBy / (event.shiftKey ? (Math.abs(increaseBy)) : 1);
                // Ensure the value stays within the range
                newValue = Math.max(885, Math.min(2000, newValue));
                input.value = newValue;
                input.dispatchEvent(new Event('input')); // Trigger the input event
            }

        } else if (keyboardMap.type === 'modes') {
            const { name, modes } = keyboardMap;
            const input = document.getElementById(name);

            if (input) {
                const currentValue = parseInt(input.value);
                const currentIndex = modes.indexOf(currentValue);

                if (currentIndex === -1) {
                    input.value = modes[0];
                } else {
                    // If the current value is in the modes array, toggle to the next mode
                    const nextIndex = (currentIndex + 1) % modes.length;
                    input.value = modes[nextIndex];
                }

                console.log(`Current value for ${name}: ${currentValue} {type of currentValue : ${typeof currentValue} ${currentIndex}}`);
                // look for the current value in the modes array
                // Toggle between off and on values
                input.value = (input.value == off) ? on : off;
                input.dispatchEvent(new Event('input')); // Trigger the input event
            }

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