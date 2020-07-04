import os
import bpy
from PIL import Image

# Constants
spriteSize = bpy.data.scenes['Scene'].render.resolution_x

# Variables
basePath = r'G:\Files\Media\Blender\Spritesheet'
imageName = 'guy'
animations = [
    {'name': 'walk', 'startFrame': 1, 'endFrame': 20}
]

for animation in animations:
    renderPaths = []
    startFrame = animation['startFrame']
    endFrame = animation['endFrame']
    name = animation['name']
    animationLength = endFrame - startFrame

    file = os.path.join(basePath, imageName)
    renderPaths.append(file)

    for i in range(animationLength):
        bpy.context.scene.frame_current = startFrame + i
        bpy.context.scene.render.filepath = file + str(i)
        bpy.ops.render.render(write_still=True)

    spriteSheet = Image.new('RGBA', (spriteSize * animationLength, spriteSize * len(renderPaths)))
    for i in range(len(renderPaths)):
        for j in range(animationLength):
            imagePath = renderPaths[i] + str(j) + '.png'
            image = Image.open(imagePath)

            spriteSheet.paste(image, (spriteSize * j, spriteSize * i))

            os.remove(imagePath)

    spriteSheet.save(basePath + '\\' + name + '.png')




