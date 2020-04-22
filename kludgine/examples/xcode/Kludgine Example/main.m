//
//  main.m
//  Kludgine Example
//
//  Created by Jonathan Johnson on 4/21/20.
//  Copyright © 2020 Jonathan Johnson. All rights reserved.
//

#import <UIKit/UIKit.h>

extern void rust_entry(void);

int main(int argc, char * argv[]) {
    rust_entry();
    return 0;
}
