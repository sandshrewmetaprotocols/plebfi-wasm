function term_animation(element, lines) {
  let i = 0;
  let currentLine = 0;
  const type = () => {
    if (currentLine >= lines.length) {
      return;
    }
    const line = lines[currentLine];
    if (i < line.length) {
      element.innerHTML += line.charAt(i);
      i++;
      setTimeout(type, 50);
    } else {
      element.innerHTML += '\n';
      i = 0;
      currentLine++;
      setTimeout(type, 100);
    }
  }
  type();
}

window.term_animation = term_animation;