/// <reference path="../../types/fresh.d.ts" />

/**
 * ASCII Art Animation Library
 *
 * Implements a wave-scanning reveal animation for ASCII art.
 * A "scanner" wave oscillates across the art, revealing characters
 * with a noise transition effect.
 */

// ANSI color codes
const COLORS = {
  RESET: "\x1b[0m",
  BOLD: "\x1b[1m",
  DIM: "\x1b[2m",
  BRIGHT_GREEN: "\x1b[92m",
};

// Characters used during the transition "glitch" phase
const NOISE_CHARS = ['·', ':', '×', 'ø', '≠', '≈', '≡'];

// Character used for unrevealed state
const PIXEL_CHAR = '●';

interface ArtPixel {
  row: number;
  col: number;
  char: string;
}

interface ParsedArt {
  pixels: ArtPixel[];
  height: number;
  width: number;
  maxCol: number;
}

/**
 * Parse ASCII art into a structured format
 */
function parseArt(art: string): ParsedArt {
  const lines = art.trim().split('\n');
  const pixels: ArtPixel[] = [];

  for (let row = 0; row < lines.length; row++) {
    const line = lines[row];
    for (let col = 0; col < line.length; col++) {
      const char = line[col];
      if (char !== ' ') {
        pixels.push({ row, col, char });
      }
    }
  }

  const maxRow = Math.max(...pixels.map(p => p.row), 0);
  const maxCol = Math.max(...pixels.map(p => p.col), 0);

  return {
    pixels,
    height: maxRow + 1,
    width: maxCol + 1,
    maxCol,
  };
}

/**
 * Configuration for the animation
 */
export interface AnimationConfig {
  /** Speed of the wave oscillation (default: 0.08) */
  waveSpeed?: number;
  /** Width of the transition band (default: 8) */
  waveWidth?: number;
  /** ANSI color code for the revealed text (default: bright green) */
  revealedColor?: string;
  /** ANSI color code for the unrevealed pixels (default: dim) */
  pixelColor?: string;
  /** ANSI color code for the transition noise (default: dim) */
  noiseColor?: string;
  /** Whether to use bold for revealed text (default: true) */
  bold?: boolean;
  /** Update interval in milliseconds (default: 50) */
  updateInterval?: number;
}

/**
 * Render a single frame of the animation
 */
function renderFrame(
  artData: ParsedArt,
  time: number,
  config: Required<AnimationConfig>
): string[] {
  const { waveWidth, revealedColor, pixelColor, noiseColor, bold } = config;

  // Initialize grid with spaces
  const grid: string[][] = [];
  for (let r = 0; r < artData.height; r++) {
    grid[r] = new Array(artData.width).fill(' ');
  }

  // Calculate wave position (oscillates left to right using sine)
  const padding = 15;
  const range = artData.maxCol + (padding * 2);
  const waveX = (Math.sin(time) * (range / 2)) + (artData.maxCol / 2);

  // Determine character for each pixel
  for (const pixel of artData.pixels) {
    const dist = pixel.col - waveX;

    let charToDraw: string;
    let color: string;

    if (dist > waveWidth) {
      // Right side: unrevealed "pixelated" mode
      charToDraw = PIXEL_CHAR;
      color = pixelColor;
    } else if (dist < -waveWidth) {
      // Left side: revealed ASCII mode
      charToDraw = pixel.char;
      color = bold ? `${COLORS.BOLD}${revealedColor}` : revealedColor;
    } else {
      // Transition band: digital noise
      const idx = Math.floor(Math.random() * NOISE_CHARS.length);
      charToDraw = NOISE_CHARS[idx];
      color = noiseColor;
    }

    grid[pixel.row][pixel.col] = `${color}${charToDraw}${COLORS.RESET}`;
  }

  // Convert grid to lines
  const lines: string[] = [];
  for (let r = 0; r < artData.height; r++) {
    lines.push(grid[r].join(''));
  }

  return lines;
}

/**
 * Animation controller for ASCII art
 */
export class AsciiAnimation {
  private artData: ParsedArt;
  private config: Required<AnimationConfig>;
  // Start at -PI/2 so sin(-PI/2) = -1, placing wave at far left (all pixelated)
  private time: number = -Math.PI / 2;
  private running: boolean = false;
  private onFrame: ((lines: string[]) => void) | null = null;

  constructor(art: string, config: AnimationConfig = {}) {
    this.artData = parseArt(art);
    this.config = {
      waveSpeed: config.waveSpeed ?? 0.03,  // Slower for smoother animation
      waveWidth: config.waveWidth ?? 10,
      revealedColor: config.revealedColor ?? COLORS.BRIGHT_GREEN,
      pixelColor: config.pixelColor ?? COLORS.DIM,
      noiseColor: config.noiseColor ?? COLORS.DIM,
      bold: config.bold ?? true,
      updateInterval: config.updateInterval ?? 33,  // ~30fps
    };
  }

  /**
   * Get the dimensions of the art
   */
  getDimensions(): { height: number; width: number } {
    return { height: this.artData.height, width: this.artData.width };
  }

  /**
   * Render a single frame at the current time
   */
  render(): string[] {
    return renderFrame(this.artData, this.time, this.config);
  }

  /**
   * Start the animation loop
   * @param callback - Called with each frame's lines
   */
  start(callback: (lines: string[]) => void): void {
    if (this.running) {
      return; // Already running
    }

    this.running = true;
    this.onFrame = callback;

    // Render initial frame
    callback(this.render());

    // Start async animation loop
    this.runLoop();
  }

  /**
   * Internal async animation loop using editor.sleep()
   */
  private async runLoop(): Promise<void> {
    while (this.running && this.onFrame) {
      await editor.sleep(this.config.updateInterval);
      if (this.running && this.onFrame) {
        this.time += this.config.waveSpeed;
        this.onFrame(this.render());
      }
    }
  }

  /**
   * Stop the animation loop
   */
  stop(): void {
    this.running = false;
    this.onFrame = null;
  }

  /**
   * Check if animation is running
   */
  isRunning(): boolean {
    return this.running;
  }

  /**
   * Reset animation time to beginning
   */
  reset(): void {
    this.time = -Math.PI / 2;
  }

  /**
   * Get static (fully revealed) version of the art
   */
  getStatic(): string[] {
    const { revealedColor, bold } = this.config;
    const color = bold ? `${COLORS.BOLD}${revealedColor}` : revealedColor;

    const grid: string[][] = [];
    for (let r = 0; r < this.artData.height; r++) {
      grid[r] = new Array(this.artData.width).fill(' ');
    }

    for (const pixel of this.artData.pixels) {
      grid[pixel.row][pixel.col] = `${color}${pixel.char}${COLORS.RESET}`;
    }

    const lines: string[] = [];
    for (let r = 0; r < this.artData.height; r++) {
      lines.push(grid[r].join(''));
    }

    return lines;
  }
}

// Export the FRESH ASCII art for convenience
export const FRESH_ART = `███████╗██████╗ ███████╗███████╗██╗  ██╗
██╔════╝██╔══██╗██╔════╝██╔════╝██║  ██║
█████╗  ██████╔╝█████╗  ███████╗███████║
██╔══╝  ██╔══██╗██╔══╝  ╚════██║██╔══██║
██║     ██║  ██║███████╗███████║██║  ██║
╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═╝`;
