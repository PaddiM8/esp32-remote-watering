const addButton = document.getElementById("add-button");
const subButton = document.getElementById("sub-button");
const secondsInput = document.getElementById("seconds-input");

const pump1Button = document.getElementById("pump1-button");
const pump2Button = document.getElementById("pump2-button");
const pump3Button = document.getElementById("pump3-button");

const startButton = document.getElementById("start-button");

let selectedButton = null;

function selectButton(button) {
    selectedButton?.classList.remove("selected");
    selectedButton = button;
    selectedButton.classList.add("selected");
}

addButton.addEventListener("click", () => {
    secondsInput.value = Number(secondsInput.value) + 10;
    if (Number(secondsInput.value) > 200) {
        secondsInput.value = 200;
    }
});

subButton.addEventListener("click", () => {
    secondsInput.value = Number(secondsInput.value) - 10;
    if (Number(secondsInput.value) < 0) {
        secondsInput.value = 0;
    }
});

pump1Button.addEventListener("click", () => selectButton(pump1Button));
pump2Button.addEventListener("click", () => selectButton(pump2Button));
pump3Button.addEventListener("click", () => selectButton(pump3Button));

startButton.addEventListener("click", async () => {
    if (!selectedButton || !secondsInput.value) {
        return;
    }

    const pumpId = selectedButton.getAttribute("data-id");
    startButton.disabled = true;
    await fetch(`/device/pump?pump=${pumpId}&duration=${secondsInput.value}`)
    startButton.disabled = false;
});
