function term_animation(id, lines) {
  const element = document.getElementById(id);
  if (!element) {
    console.error(`Terminal element with id "${id}" not found.`);
    return;
  }
  let i = 0;
  let currentLine = 0;
  element.innerHTML = '';
  const type = () => {
    if (currentLine >= lines.length) {
      return;
    }
    const line = lines[currentLine];
    if (line.startsWith('$')) {
      if (i < line.length) {
        element.innerHTML += line.charAt(i);
        i++;
        setTimeout(type, 20); // Faster typing
      } else {
        element.innerHTML += '\n';
        i = 0;
        currentLine++;
        setTimeout(type, 50);
      }
    } else {
      element.innerHTML += line + '\n';
      currentLine++;
      setTimeout(type, 50);
    }
  }
  type();
}

window.term_animation = term_animation;