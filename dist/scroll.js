function autoScroll(element) {
  if (element.scrollHeight <= element.clientHeight) {
    return;
  }

  element.scrollTop = 0;
  let scrollDelay = 3000; // 3 second delay before starting
  const pixelsPerSecond = 12.5;
  let scrollDirection = 1; // 1 for down, -1 for up
  let hasScrolledDown = false;
  let isPaused = false;

  let scrollInterval;
  let scrollTimeout;

  const startScrolling = () => {
    const scrollHeight = element.scrollHeight - element.clientHeight;
    const duration = (scrollHeight / pixelsPerSecond) * 1000;
    const intervalTime = 20; // ms
    const scrollStep = scrollHeight / (duration / intervalTime);

    scrollInterval = setInterval(() => {
      if (isPaused) return;

      if (scrollDirection === 1) {
        if (element.scrollTop + element.clientHeight >= element.scrollHeight - 1) {
          scrollDirection = -1;
          hasScrolledDown = true;
        }
      } else {
        if (element.scrollTop <= 1) {
          if (hasScrolledDown) {
            clearInterval(scrollInterval);
            return;
          }
          scrollDirection = 1;
        }
      }
      element.scrollTop += scrollDirection * scrollStep;
    }, intervalTime);
  };

  scrollTimeout = setTimeout(startScrolling, scrollDelay);

  element.addEventListener('mouseenter', () => {
    isPaused = true;
    element.style.scrollbarWidth = 'auto'; /* Firefox */
    element.style.msOverflowStyle = 'auto'; /* IE 10+ */
  });

  element.addEventListener('mouseleave', () => {
    isPaused = false;
    element.style.scrollbarWidth = 'none'; /* Firefox */
    element.style.msOverflowStyle = 'none';  /* IE 10+ */
  });
}

window.autoScroll = autoScroll;