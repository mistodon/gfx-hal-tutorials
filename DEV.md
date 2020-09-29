Each part checklist
===
-   [ ] Are imports sensible?
-   [ ] Are the shaders near the top?
-   [ ] Does each shader have a filename comment?
-   [ ] Is every code block introduced with a colon?
-   [ ] Does the final code file have appropriate line spacing?
-   [ ] Are there any unexplained unwraps/expects?
-   [ ] Are there any unnecessary // ... comments?

Specific TODOs
===

## Imminent
-   [ ] Fix up imports to avoid long type paths in parts 1 through 4

## Eventually
-   [x] Use ManuallyDrop! It's stable now!
-   [ ] Rename all `skybox` to `sky`
-   [ ] Rename all `rtt` to something more descriptive
-   [ ] Rename all `postprocess` to `toon`
-   [ ] Add a comment to each shader block showing the filename

Plan
===

- Part 7: Indexed drawing
    - Draw a skybox using an index buffer
    - "while we're at it" load a sky texture into another desc set
    - Conclusion:
        - Looks nice and now we know how to draw multiple kinds of thing
        - But the skybox shouldn't be lit! We need to use a different shader
        - But how?? Find out next time!
- Part 8: Multiple pipelines
    - Have a nice new shader for the skybox
    - Might involve a new desc set layout?
    - Make a new pipeline for it
    - Make the skybox clear the screen, but the other objects not
    - Conclusion:
        - Definitely looks nicer, but I'm getting vertigo!
        - Some ground would really make this place more grounded.
        - That doesn't sound too interesting in and of itself, but
        - there's some interesting stuff you can do with ground
- Part 9: Stencil buffer
    - OK this is getting a little contrived, but
    - Draw a box for the ground that occludes part of the teapot
    - But use the stencil buffer to draw a silhouette of the teapot through
    - Conclusion:
        - Now that I can see half a spinning teapot with x-ray vision,
        - I'm really starting to feel at home.
        - But occluding the teapot has got me thinking about things I can't see
        - I wonder if we could expand our view of the scene somehow, without
        - having to write any complicated code to move a camera, of course.
        - Just something to reflect on for next time!
- Part 10: Render-to-texture
    - Like many great astronauts, I want to see the dark side of this teapot.
    - But I already have my viewport set up real nice an simple, and I don't
    - want to stare right into the light
    - So let's add a mirror behind the teapot and get the best of both worlds!
    - Conclusion:
        - Hope you liked this contrived example.
        - I think this will be the end, for now.
        - Call it Season 1 of these tutorials.
        - I have a few ideas for more, but they'll have to wait.
    - Things I learned:
        1.  Never make two of something if one will do:
            - Don't make two textures if you can atlas them together
            - Don't make two descriptor sets if you can just use different Push Cs
            - Don't make two vertex buffers if you can put your meshes into one
            - Don't write two shaders if you can tweak the inputs of one
            - Don't make two pipelines if one will cover it

Future topics
===

1.  Indexed drawing
2.  Instanced drawing
3.  Multiple render (sub-)passes
4.  Multiple vertex buffers
5.  Multiple vertex buffer types
6.  Multiple descriptor sets
7.  Multiple pipelines
8.  Multiple textures/texture array
9.  Cube maps
10. Compute pipelines?
11. OpenGL support
12. WebGL/WASM support

# Handy dandy links:
1.  Don't write descriptor sets in a command buffer or you'll get a race condition:
    - https://community.khronos.org/t/what-does-vkupdatedescriptorsets-exactly-do/7184
