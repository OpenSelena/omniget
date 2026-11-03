import { describe, expect, it } from "vitest";
import { mediaKind } from "./media-kind";

describe("mediaKind", () => {
  it("reads the extension from Windows and Unix paths", () => {
    expect(mediaKind("C:\\Users\\me\\Clip.MP4")).toBe("video");
    expect(mediaKind("/home/me/song.opus")).toBe("audio");
    expect(mediaKind("/home/me/snap_001.jpg")).toBe("image");
  });
  it("does not trust dots in folder names", () => {
    expect(mediaKind("/home/me.mp4/readme")).toBe("other");
    expect(mediaKind("/home/me/.mp4")).toBe("other");
  });
});
