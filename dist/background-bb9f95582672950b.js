const canvas = document.getElementById('background');
const ctx = canvas.getContext('2d');

canvas.width = window.innerWidth;
canvas.height = window.innerHeight;

let sprites = [];

function drawCrystal(x, y, size) {
  ctx.beginPath();
  ctx.moveTo(x, y - size);
  ctx.lineTo(x + size / 2, y);
  ctx.lineTo(x, y + size);
  ctx.lineTo(x - size / 2, y);
  ctx.closePath();
  ctx.stroke();
}

function drawSnowflake(x, y, size) {
  const arms = 6;
  for (let i = 0; i < arms; i++) {
    const angle = (Math.PI * 2 / arms) * i;
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(x + Math.cos(angle) * size, y + Math.sin(angle) * size);
    ctx.stroke();
  }
}

function drawBitcoin(x, y, size) {
  ctx.font = `${size}px Arial`;
  ctx.fillText('₿', x, y);
}

function resize() {
  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;
  sprites = [];
  for (let i = 0; i < 50; i++) {
    let type;
    const rand = Math.random();
    if (rand < 0.33) {
      type = 'crystal';
    } else if (rand < 0.66) {
      type = 'snowflake';
    } else {
      type = 'bitcoin';
    }
    sprites.push({
      x: Math.random() * canvas.width,
      y: Math.random() * canvas.height,
      size: Math.random() * 20 + 10,
      speed: Math.random() * 0.5 + 0.2,
      type: type
    });
  }
}

window.addEventListener('resize', resize);
resize();

function animate() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  const gradient = ctx.createLinearGradient(0, 0, canvas.width, canvas.height);
  gradient.addColorStop(0, '#485563');
  gradient.addColorStop(1, '#29323c');
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  ctx.strokeStyle = 'rgba(255, 255, 255, 0.5)';
  ctx.lineWidth = 2;

  for (const sprite of sprites) {
    sprite.y -= sprite.speed;
    if (sprite.y < -sprite.size) {
      sprite.y = canvas.height + sprite.size;
      sprite.x = Math.random() * canvas.width;
    }
    if (sprite.type === 'crystal') {
      drawCrystal(sprite.x, sprite.y, sprite.size);
    } else if (sprite.type === 'snowflake') {
      drawSnowflake(sprite.x, sprite.y, sprite.size);
    } else {
      ctx.fillStyle = 'rgba(255, 255, 255, 0.5)';
      drawBitcoin(sprite.x, sprite.y, sprite.size);
    }
  }

  requestAnimationFrame(animate);
}

animate();